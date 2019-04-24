<template>
  <div class="card">
    <header class="card-header">
      <p class="card-header-title">
        Processing Tree
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
import { MatcherConfigDto } from '../generated/dto';

@Component({
  components: {
    DisplayAsJson,
  },
})
export default class ProcessingTree extends Vue {
  @Prop() public tree!: MatcherConfigDto;
}
</script>
