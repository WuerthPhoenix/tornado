import { shallowMount } from '@vue/test-utils';
import ProcessingTree from '@/components/ProcessingTree.vue';
import { MatcherConfigDto } from '@/generated/dto';

describe('Tornado.vue', () => {
  it('renders props.msg when passed', () => {
    const tree: MatcherConfigDto = {
      type: 'Rules',
      rules: [],
    };
    const wrapper = shallowMount(ProcessingTree, {
      propsData: { tree },
      stubs: ['Tree'],
    });
    expect(wrapper.text()).toBeDefined();
  });
});
