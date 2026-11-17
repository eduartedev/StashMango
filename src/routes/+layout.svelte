<script>
  import "../app.css";
  import "@fontsource/fraunces/600.css";
  import "@fontsource/nunito-sans/400.css";
  import "@fontsource/nunito-sans/700.css";
  import "@fontsource/fragment-mono/400.css";
  let { children } = $props();
</script>

<svelte:window oncontextmenu={(event) => event.preventDefault()} />

{@render children()}
