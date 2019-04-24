<template>
  <div>
    <h1 class="title">Tornado</h1>
    <p class="subtitle">
      Config your
      <strong>Tornado</strong> instance!
    </p>
    <div class="columns">
      <div class="column is-two-thirds">
        <ProcessingTree :tree="tree"/>
      </div>
      <div class="column">
        <NodeDetails/>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { Component, Vue } from 'vue-property-decorator';
import NodeDetails from '@/components/NodeDetails.vue';
import ProcessingTree from '@/components/ProcessingTree.vue';
import { MatcherConfigDto } from '../generated/dto';

@Component({
  components: {
    ProcessingTree,
    NodeDetails,
  },
})
export default class Tornado extends Vue {

  public tree: MatcherConfigDto = {
    type: 'Filter',
    nodes: {},
    filter: {
      name: 'filter name',
      description: 'great filter',
      active: true,
      filter: {
        type: 'equal',
        first: 'value one',
        second: 'value two',
      },
    },
  };


}
</script>
