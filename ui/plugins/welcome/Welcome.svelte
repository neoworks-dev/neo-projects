<script lang="ts">
  // Before any workspace is open: pick the folder that holds your projects' notes and canvases.
  // An Obsidian vault works as it is.
  import { files } from "@neoworks/svelte-gpui";
  import { Button, Input, Text, View } from "@neoworks/svelte-gpui/ui";
  import { app } from "../../lib/state.svelte.ts";
  import { button, buttonHover, c, field, fieldFocus } from "../../lib/theme.ts";

  let path = $state("");

  async function pick() {
    const picked = await files.pickFiles({ directories: true, prompt: "Choose a workspace folder" });
    if (picked?.[0]) await app.openWorkspace(picked[0]);
  }
</script>

<View style={{ flex: 1, align: "center", justify: "center" }}>
  <View style={{ width: 460, gap: 14 }}>
    <Text style={{ fontSize: 22, fontWeight: "semibold" }}>Neoworks Projects</Text>
    <Text style={{ fontSize: 13, color: c.muted, lineHeight: 20 }}>
      Choose a folder for your workspace. Your notes and canvases are plain files in it; GitHub stays the place
      for issues. An existing Obsidian vault is fine.
    </Text>
    <Button style={{ ...button, alignSelf: "start" }} hover={buttonHover} onpress={() => void pick()}>
      <Text>Choose folder…</Text>
    </Button>
    <View style={{ direction: "row", gap: 8 }}>
      <Input
        bind:value={path}
        placeholder="or type a path, e.g. ~/Projects"
        style={{ ...field, flex: 1 }}
        focus={fieldFocus}
        onsubmit={() => path.trim() && void app.openWorkspace(path.trim())}
      />
      <Button style={button} hover={buttonHover} onpress={() => path.trim() && void app.openWorkspace(path.trim())}>
        <Text>Open</Text>
      </Button>
    </View>
  </View>
</View>
