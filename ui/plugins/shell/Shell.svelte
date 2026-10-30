<script lang="ts">
  // The window, laid out as Android Studio's islands: a slim top bar, a tool rail at the left, the
  // main view and the panel for what is selected as rounded islands on a darker frame, and a status
  // bar at the bottom. Where each pane sits is decided here and nowhere else.
  import { Presence, Text, View } from "@neoworks/svelte-gpui/ui";
  import { fade, fly } from "svelte/transition";
  import Slot from "../../kernel/slots/Slot.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { c, radii, shadow } from "../../lib/theme.ts";
</script>

<View style={{ flex: 1, background: c.frame, color: c.text, fontFamily: "sans", fontSize: 13 }}>
  <View style={{ direction: "row", align: "center", height: 42, paddingX: 10, gap: 6, shrink: 0 }}>
    <View style={{ direction: "row", align: "center", gap: 4, flex: 1, minWidth: 0 }}>
      <Slot name="shell.top" />
    </View>
    <View style={{ direction: "row", align: "center", gap: 4 }}>
      <Slot name="shell.top.right" />
    </View>
  </View>
  <View style={{ flex: 1, direction: "row", minHeight: 0, paddingRight: 6, gap: 6 }}>
    <View style={{ width: 46, shrink: 0, align: "center", gap: 6, paddingTop: 2 }}>
      <Slot name="shell.rail" />
    </View>
    <View
      style={{
        flex: 1,
        minWidth: 0,
        radius: radii.panel,
        overflow: "hidden",
        borderWidth: 1,
        borderColor: c.borderSoft,
        background: c.bg,
      }}
    >
      <Slot name="shell.main" />
      <Slot name="shell.main.over" />
    </View>
    <Slot name="shell.panel" />
  </View>
  <View style={{ direction: "row", align: "center", height: 28, paddingX: 12, gap: 8, shrink: 0 }}>
    <View style={{ direction: "row", align: "center", gap: 4, flex: 1, minWidth: 0 }}>
      <Slot name="shell.status" />
    </View>
    <View style={{ direction: "row", align: "center", gap: 4 }}>
      <Slot name="shell.status.right" />
    </View>
  </View>
  {#if app.toast}
    <Presence
      in={[fly, { y: 12, duration: 220 }]}
      out={[fade, { duration: 160 }]}
      style={{ position: "absolute", bottom: 44, right: 18, maxWidth: 420 }}
    >
      <View
        style={{
          direction: "row",
          gap: 10,
          paddingX: 14,
          paddingY: 11,
          radius: radii.card,
          background: c.surface,
          borderWidth: 1,
          borderColor: c.border,
          shadow,
        }}
      >
        <View style={{ width: 8, height: 8, radius: 4, marginTop: 5, background: c.danger, shrink: 0 }} />
        <Text style={{ fontSize: 13, color: c.text, lineHeight: 19 }}>{app.toast}</Text>
      </View>
    </Presence>
  {/if}
  <Slot name="shell.overlay" />
</View>
