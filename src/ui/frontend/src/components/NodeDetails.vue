<template>
  <div class="card">
    <header class="card-header">
      <p class="card-header-title">
        Node Details
      </p>
    </header>
    <div class="card-content">
      <div class="content">
        <DisplayAsJson :obj="tree"/>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from 'vue-property-decorator';
import DisplayAsJson from '@/components/DisplayAsJson.vue';

@Component({
  components: {
    DisplayAsJson,
  },
})
export default class NodeDetails extends Vue {
  @Prop() public tree!: any;
}
</script>
