<template>
  <div id="app">
    <div id="nav">
      <router-link to="/">Home</router-link> |
      <router-link to="/tornado">Tornado</router-link> |
      <router-link to="/tornado_test_event">Test Event</router-link>
    </div>
    <div class="section">
      <div class="container">
        <router-view/>
      </div>
    </div>
  </div>
</template>

<style lang="scss">

@import 'bulma';

#nav {
  padding: 30px;
  a {
    font-weight: bold;
    color: #2c3e50;
    &.router-link-exact-active {
      color: #42b983;
    }
  }
}
</style>
