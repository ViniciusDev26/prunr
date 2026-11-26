<script>
  import { goto } from "$app/navigation";
  import Card from "$components/Card.svelte";
  import { format } from "$lib/format";
</script>

<Card title={format("# hi")} on:click={() => goto("/about")} />
