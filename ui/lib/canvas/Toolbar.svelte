<script lang="ts" module>
  export interface Tool {
    icon: string;
    label: string;
    /** The key that does the same, shown in the tooltip. */
    key?: string;
    /** A mode that is on, such as the pen while drawing. */
    active?: boolean;
    onpress: () => void;
  }
</script>

<script lang="ts">
  // The canvas's floating toolbar, at the bottom centre: what can be added, then the zoom with
  // its percentage and Fit.
  import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
  import { Tooltip } from "@neoworks/svelte-gpui-ui";
  import { c, radii, shadow } from "../theme.ts";
  import { glide, zoomedAbout } from "./motion.ts";
  import type { ViewState } from "./Viewport.svelte";
  import { icons } from "../icons.ts";
  const { fit: corners, remove: minus, add: plus } = icons;

  interface Props {
    tools: Tool[];
    view: ViewState;
    /** The viewport's size, to zoom about its centre. */
    size: { width: number; height: number };
    onfit: () => void;
  }

  let { tools, view = $bindable(), size, onfit }: Props = $props();

  function zoomTo(zoom: number) {
    glide(view, zoomedAbout(view, zoom, size), (next) => (view = next), 180);
  }

  const iconButton = {
    size: 34,
    radius: 10,
    align: "center",
    justify: "center",
    color: c.muted,
    transition: 120,
  } as const;
  const iconHover = { background: c.hover, color: c.text };
</script>

<View
  style={{
    position: "absolute",
    bottom: 16,
    left: 0,
    right: 0,
    align: "center",
  }}
>
  <View
    style={{
      direction: "row",
      align: "center",
      gap: 2,
      padding: 4,
      radius: radii.card + 2,
      background: c.surface,
      borderWidth: 1,
      borderColor: c.border,
      shadow,
    }}
    onpress={() => {}}
  >
    {#each tools as tool (tool.label)}
      <Tooltip text={tool.key ? `${tool.label}  ${tool.key}` : tool.label} placement="top">
        <Button
          style={tool.active ? { ...iconButton, background: c.accent, color: c.onAccent } : iconButton}
          hover={tool.active ? {} : iconHover}
          onpress={tool.onpress}
        >
          <Icon svg={tool.icon} size={18} />
        </Button>
      </Tooltip>
    {/each}
    <View style={{ width: 1, height: 20, background: c.border, marginX: 4 }} />
    <Tooltip text="Zoom out" placement="top">
      <Button style={iconButton} hover={iconHover} onpress={() => zoomTo(view.zoom / 1.25)}>
        <Icon svg={minus} size={15} />
      </Button>
    </Tooltip>
    <Tooltip text="Actual size" placement="top">
      <Button
        style={{ ...iconButton, width: 52 }}
        hover={iconHover}
        onpress={() => zoomTo(1)}
      >
        <Text style={{ fontSize: 12, fontFamily: "mono" }}>{Math.round(view.zoom * 100)}%</Text>
      </Button>
    </Tooltip>
    <Tooltip text="Zoom in" placement="top">
      <Button style={iconButton} hover={iconHover} onpress={() => zoomTo(view.zoom * 1.25)}>
        <Icon svg={plus} size={15} />
      </Button>
    </Tooltip>
    <Tooltip text="Fit everything" placement="top">
      <Button style={iconButton} hover={iconHover} onpress={onfit}>
        <Icon svg={corners} size={17} />
      </Button>
    </Tooltip>
  </View>
</View>
