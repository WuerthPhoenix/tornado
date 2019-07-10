<template>
  <div>
    <Tree :data="[treeData]" />
    <DisplayAsJson :obj="tree"/>
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

  get treeData(): Patterns.TreeCard {
    const treeCard: Patterns.TreeCard = this.toTreeNode('root', this.tree);
    // console.log(treeCard);
    return treeCard;
  }

  private toTreeNode(name: string, node: MatcherConfigDto): Patterns.TreeCard {
    if (node.type === 'Filter') {
      const treeNode: Patterns.TreeCard = {
        active: true,
        id: 0,
        title: `Filter - ${name}` + node.filter.name,
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
        id: 0,
        active: true,
        title: `Rules - ${name}`,
        description: `Rule set with ${node.rules.length} rules`,
        actions: [],
        children: [],
      };
      return treeNode;
    }
  }
}
</script>
