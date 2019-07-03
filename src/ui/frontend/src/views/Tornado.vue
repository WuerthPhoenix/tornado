<template>
  <div>
    <h1 class="title">Tornado</h1>
    <p class="subtitle">
      Config your
      <strong>Tornado</strong> instance!
    </p>
    <Tree :data="[{
      active: 3,
      nodes: [
        {
          id: 1,
          title: 'Email',
          description: 'Configure events delivered from a received email'
        }
       ]
    }]" />
    <div class="columns">
      <div class="column">
        <button class="button" @click="loadConfig()">Load config from Tornado</button>
      </div>
    </div>
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
import { MatcherConfigDto } from '@/generated/dto';
import configModule from '@/store/module/config';

@Component({
  components: {
    ProcessingTree,
    NodeDetails,
  },
})
export default class Tornado extends Vue {

  get tree(): MatcherConfigDto {
    return configModule.config;
  }

  public loadConfig() {
    configModule.getConfig();
  }

}
</script>
