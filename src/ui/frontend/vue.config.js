module.exports = {
    devServer: {
      proxy: {
        '/neteye/tornado/backend/api': {
          target: 'http://127.0.0.1:4748',
          ws: true,
          changeOrigin: true,
          pathRewrite: {
            '^/neteye/tornado/backend/api': '/api'
          }
        }
      }
    },
    transpileDependencies: ['vuex-module-decorators']
  }
