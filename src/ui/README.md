 
# Tornado UI Spike

## Dto

## Backend

## UI

Built with [vue cli 3](https://cli.vuejs.org).

Built time dependencies:
- node 10.x 
- vue cli 3 globally installed
- chrome is required for unit and e2e tests execution

Install vue cli:
> npm install -g @vue/cli

Check correct version is installed, it should be 3.x: 
> vue --version

### Build the UI:
the build.sh file perform the frontend build. The process involves 4 steps:
1. Copy the dto.ts files in the `generated` folder
1. execute the linter
1. execute the unit tests
1. build the package into the dist folder


At runtime, only the content of the dist folder is required.

### install dependencies
> npm install

### serve the frontend locally
> npm run serve

