<script lang="ts">
  // The window: a top bar, the main view (overview or a project) and a panel at the right for what
  // is selected. Where each pane sits is decided here and nowhere else.
  import { Presence, Text, View } from "@neoworks/svelte-gpui/ui";
  import { fade, fly } from "svelte/transition";
  import Slot from "../../kernel/slots/Slot.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { c } from "../../lib/theme.ts";
</script>

<View style={{ flex: 1, background: c.bg, color: c.text, fontFamily: "sans", fontSize: 13 }}>
  <View
    style={{
      direction: "row",
      align: "center",
      height: 44,
      paddingX: 12,
      gap: 8,
      shrink: 0,
      borderBottomWidth: 1,
      borderColor: c.border,
      background: c.surface,
    }}
  >
    <View style={{ direction: "row", align: "center", gap: 6, flex: 1, minWidth: 0 }}>
      <Slot name="shell.top" />
    </View>
    <View style={{ direction: "row", align: "center", gap: 6 }}>
      <Slot name="shell.top.right" />
    </View>
  </View>
  <View style={{ flex: 1, direction: "row", minHeight: 0 }}>
    <View style={{ flex: 1, minWidth: 0 }}>
      <Slot name="shell.main" />
    </View>
    <Slot name="shell.panel" />
  </View>
  {#if app.toast}
    <Presence
      in={[fly, { y: 8, duration: 150 }]}
      out={[fade, { duration: 150 }]}
      style={{ position: "absolute", bottom: 16, left: 16, right: 16, align: "center" }}
    >
      <View
        style={{
          paddingX: 14,
          paddingY: 8,
          radius: 8,
          background: c.raised,
          borderWidth: 1,
          borderColor: c.danger,
          maxWidth: 640,
        }}
      >
        <Text style={{ fontSize: 12, color: c.text }}>{app.toast}</Text>
      </View>
    </Presence>
  {/if}
  <Slot name="shell.overlay" />
</View>
