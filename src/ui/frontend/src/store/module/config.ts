import { VuexModule, Module, getModule, MutationAction } from 'vuex-module-decorators';
import Store from '..';
import { MatcherConfigDto } from '@/generated/dto';
import { getConfig } from '@/api/api';

@Module({
    dynamic: true,
    store: Store,
    name: 'config',
    namespaced: true,
})
default class ConfigModule extends VuexModule {

    public config: MatcherConfigDto = {
        type: 'Rules',
        rules: [],
    };

    public loaded = false;

    @MutationAction({ mutate: ['config', 'loaded'] })
    public async getConfig() {
        const config = await getConfig();
        return {
            config,
            loaded: true,
        };
    }

}

export default getModule(ConfigModule);
