<script setup lang="ts">
import { useCounter } from "~/composables/counter";
import { useRoute } from "#imports";
import { shared } from "~~/shared";

const { count } = useCounter();
const route = useRoute();
</script>

<template>
  <Greeting :count="count" :path="route.path" :shared="shared" />
</template>
