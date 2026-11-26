<script context="module">
  import { helper } from "./helper.js";
</script>

<script lang="ts">
  import dayjs from "dayjs";
  let now: string = dayjs().format();
</script>

<p>{now} {helper()}</p>
