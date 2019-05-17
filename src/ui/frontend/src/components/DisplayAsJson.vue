<template>
  <pre>{{ obj | asJson }}</pre>
</template>

<script lang="ts">
import { Component, Prop, Vue } from 'vue-property-decorator';

@Component({
  filters: {
    asJson: (value: any) => {
      return JSON.stringify(value, null, 2);
    },
  },
})
export default class ViewAsJson extends Vue {
  @Prop() public obj!: any;
}
</script>
