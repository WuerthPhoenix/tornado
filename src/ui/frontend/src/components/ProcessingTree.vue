<template>
  <div>
    <Tree class="wptree" :data="treeData" :config="treeConfig" :key="Math.random()">
      <Placeholder>Could not open your Tornado configuration.</Placeholder>
    </Tree>
    <!-- <DisplayAsJson :obj="treeData"/> -->
  </div>
</template>

<script lang="ts">
import { Component, Prop, Vue } from 'vue-property-decorator';
import DisplayAsJson from '@/components/DisplayAsJson.vue';
import { MatcherConfigDto } from '../generated/dto';
import { Patterns } from 'wp-design-system';

@Component({
  components: {
    DisplayAsJson,
  },
})
export default class ProcessingTree extends Vue {
  @Prop() public tree!: MatcherConfigDto;
  @Prop() public loaded!: boolean;

  public count = 0;

  get treeData(): Patterns.TreeData {
    const treeCard: Patterns.TreeCard = this.toTreeNode('root', this.tree);
    return {cards: [treeCard]};
  }

  get treeConfig(): Patterns.TreeConfig {
    return {
      class: 'wptree',
      loaded: this.loaded,
    };
  }

  private toTreeNode(name: string, node: MatcherConfigDto): Patterns.TreeCard {
    if (node.type === 'Filter') {
      const treeNode: Patterns.TreeCard = {
        active: false,
        id: this.count++,
        title: `${node.filter.name}`,
        description: node.filter.description,
        actions: [],
        children: [],
      };

      Object.keys(node.nodes).forEach((key) => {
        treeNode.children.push(this.toTreeNode(key, node.nodes[key]));
      });

      return treeNode;
    } else {
      const treeNode: Patterns.TreeCard = {
        id: this.count++,
        active: false,
        title: `${name}`,
        description: ``,
        actions: [],
        children: [],
        info: {
          field1: 'Rule set',
          field2: `${node.rules.length} rules`,
        },
      };
      return treeNode;
    }
  }
}
</script>
