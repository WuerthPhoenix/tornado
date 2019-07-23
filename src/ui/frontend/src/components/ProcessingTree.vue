<template>
  <div>
    <Tree class="wptree" :data="treeData" :key="Math.random()">
      <Placeholder>Your processing tree has no nodes.</Placeholder>
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

  public count = 0;

  get treeData(): Patterns.TreeData {
    const treeCard: Patterns.TreeCard = this.toTreeNode('root', this.count++, this.tree);
    return {cards: [treeCard]};
  }

  private toTreeNode(name: string, id: number, node: MatcherConfigDto): Patterns.TreeCard {
    if (node.type === 'Filter') {
      const treeNode: Patterns.TreeCard = {
        active: false,
        id,
        title: `${name} ${node.filter.name}`,
        description: node.filter.description,
        actions: [],
        children: [],
      };

      Object.keys(node.nodes).forEach((key) => {
        id = id + 1;
        treeNode.children.push(this.toTreeNode(key, id, node.nodes[key]));
      });

      return treeNode;
    } else {
      const treeNode: Patterns.TreeCard = {
        id,
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
