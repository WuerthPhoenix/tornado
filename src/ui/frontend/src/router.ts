import Vue from 'vue';
import Router from 'vue-router';
import Tornado from '@/views/Tornado.vue';

Vue.use(Router);

export default new Router({
  routes: [
    {
      path: '/',
      name: 'tornado',
      component: Tornado,
    },
    {
      path: '/tornado_test_event',
      name: 'tornado_test_event',
      component: () => import('@/views/TornadoTestEvent.vue'),
    },
  ],
});
