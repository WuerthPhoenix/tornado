import Vue from 'vue';
import Router from 'vue-router';
import Home from './views/Home.vue';

Vue.use(Router);

export default new Router({
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home,
    },
    {
      path: '/tornado',
      name: 'tornado',
      component: () => import('@/views/Tornado.vue'),
    },
    {
      path: '/tornado_test_event',
      name: 'tornado_test_event',
      component: () => import('@/views/TornadoTestEvent.vue'),
    },
  ],
});
