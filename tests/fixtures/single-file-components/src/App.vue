<template>
  <Child :value="used" /> <!-- <script>import "nope"</script> is a comment of the template -->
</template>

<script setup lang="ts">
import { ref } from "vue";
import Child from "./Child.vue";
import { used } from "./util";

const count: number = ref(0).value;
</script>

<style scoped>
.é { color: red; }
</style>
