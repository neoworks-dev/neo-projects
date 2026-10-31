//! Strokes drawn on a canvas's background, kept under the canvas's own `neoworks` object.

use np_core::workspace::{Canvas, Stroke, read_canvas, write_canvas};
use serde_json::json;

fn stroke(id: &str) -> Stroke {
    Stroke { id: id.into(), color: "#f2555a".into(), width: 3.0, points: vec![0.0, 0.0, 10.5, 4.0, 20.0, 8.0] }
}

#[test]
fn strokes_come_back_from_the_file_and_leave_the_rest_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.canvas");
    let text = json!({
        "nodes": [{"id": "a", "type": "text", "text": "hi", "x": 0, "y": 0, "width": 100, "height": 50}],
        "edges": [],
        "neoworks": {"pinned": true},
        "metadata": {"version": "1.0-1.0"},
    });
    std::fs::write(&path, text.to_string()).unwrap();

    let mut canvas = read_canvas(&path).unwrap();
    canvas.set_strokes(&[stroke("s1"), stroke("s2")]).unwrap();
    write_canvas(&path, &canvas).unwrap();

    let again = read_canvas(&path).unwrap();
    assert_eq!(again.strokes(), vec![stroke("s1"), stroke("s2")]);
    assert_eq!(again.nodes.len(), 1);
    assert_eq!(again.extra["neoworks"]["pinned"], true);
    assert_eq!(again.extra["metadata"]["version"], "1.0-1.0");
}

#[test]
fn a_canvas_without_strokes_has_none() {
    assert_eq!(Canvas::default().strokes(), vec![]);
}

#[test]
fn a_stroke_that_does_not_read_is_left_out() {
    let canvas: Canvas = serde_json::from_value(json!({
        "neoworks": {"strokes": [
            {"id": "ok", "color": "#fff", "width": 2, "points": [0, 0, 1, 1]},
            {"id": "broken", "points": "nope"},
        ]},
    }))
    .unwrap();
    let ids: Vec<String> = canvas.strokes().into_iter().map(|s| s.id).collect();
    assert_eq!(ids, vec!["ok"]);
}

#[test]
fn clearing_the_strokes_writes_an_empty_list() {
    let mut canvas = Canvas::default();
    canvas.set_strokes(&[stroke("s1")]).unwrap();
    canvas.set_strokes(&[]).unwrap();
    assert_eq!(canvas.strokes(), vec![]);
    assert_eq!(canvas.extra["neoworks"]["strokes"], json!([]));
}
