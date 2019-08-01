import Vue from 'vue';
import App from './App.vue';
import router from './router';
import store from './store';
import DesignSystem from 'wp-design-system';
import 'wp-design-system/dist/system/system.css';

Vue.config.productionTip = false;

Vue.use(DesignSystem as any);

const tornadoApp = new Vue({
  router,
  store,
  render: (h) => h(App),
}).$mount('#tornado-app');
