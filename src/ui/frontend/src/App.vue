<template>
  <div>
    <Container>
      <div id="nav">
        <router-link to="/">Tornado</router-link> |
        <router-link to="/tornado_test_event">Test Event</router-link>
      </div>
    </Container>
    <router-view/>
  </div>
</template>

<style lang="scss">

@import 'bulma';

#nav {
  a {
    font-weight: bold;
    color: #2c3e50;
    &.router-link-exact-active {
      color: #42b983;
    }
  }
}
</style>
