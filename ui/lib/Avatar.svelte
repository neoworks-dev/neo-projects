<script lang="ts">
  // A person as a round picture: their GitHub avatar, over their initials, which show while it
  // loads and stay for people GitHub doesn't know (assignees of local issues).
  import { Image, Text, View } from "@neoworks/svelte-gpui/ui";
  import { projectColor } from "./theme.ts";

  interface Props {
    login: string | null;
    size?: number;
    /** Look the login up on GitHub. Off for local issues, whose names are free text. */
    github?: boolean;
    /** Drawn as a ring around it, to separate overlapping avatars. */
    outline?: string;
  }

  let { login, size = 20, github = true, outline }: Props = $props();

  let failed = $state(false);
  const tint = $derived(projectColor(login ?? "?"));
</script>

<View
  style={{
    size,
    radius: size / 2,
    align: "center",
    justify: "center",
    background: tint.line,
    borderWidth: outline ? 2 : 0,
    borderColor: outline ?? "transparent",
    overflow: "hidden",
    shrink: 0,
  }}
>
  <Text style={{ fontSize: size * 0.4, fontWeight: "medium", color: "#ffffff" }}>{(login ?? "?").slice(0, 2).toUpperCase()}</Text>
  {#if github && login && !failed}
    <Image
      src={`https://avatars.githubusercontent.com/${login}?s=${Math.round(size * 2)}`}
      fit="cover"
      style={{ position: "absolute", inset: 0, radius: size / 2 }}
      onerror={() => (failed = true)}
    />
  {/if}
</View>
