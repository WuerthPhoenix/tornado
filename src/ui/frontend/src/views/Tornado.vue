<template>
  <Container>
    <Heading>Tornado</Heading>
    <div>
      <ProcessingTree :tree="tree"/>
    </div>
  </Container>
</template>

<script lang="ts">
import { Component, Vue } from 'vue-property-decorator';
import ProcessingTree from '@/components/ProcessingTree.vue';
import { MatcherConfigDto } from '@/generated/dto';
import configModule from '@/store/module/config';

@Component({
  components: {
    ProcessingTree,
  },
})
export default class Tornado extends Vue {

  get tree(): MatcherConfigDto {
    return configModule.config;
  }
  /*
  public loadConfig() {
    configModule.getConfig();
  }
  */
  public async mounted() {
    configModule.getConfig();
  }
}
</script>
