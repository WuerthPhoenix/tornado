<template>
  <div class="card">
    <header class="card-header">
      <p class="card-header-title">
        Processing Tree
      </p>
    </header>
    <div class="card-content">
      <div class="content">
        <Tree :data="[treeData]" />
        <DisplayAsJson :obj="tree"/>
      </div>
    </div>
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

  get treeData(): Patterns.TreeData {
    const treeNode: Patterns.TreeNode = this.toTreeNode('root', this.tree);
    return {
      active: 0,
      nodes: [treeNode],
    };
  }

  private toTreeNode(name: string, node: MatcherConfigDto): Patterns.TreeNode {
    if (node.type === 'Filter') {
      const treeNode: Patterns.TreeNode = {
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
      const treeNode: Patterns.TreeNode = {
        id: 0,
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
