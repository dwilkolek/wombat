import { writable } from 'svelte/store';
import { execute } from './error-store';
import type { AwsEnv, UserConfig } from './types';
import { homeStore } from './home-store';

const createUserStore = () => {
	const loggedIn = writable(false);
	const { subscribe, set } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		tracked_names: [],
		known_profiles: [],
		last_used_profile: undefined,
		preffered_environments: []
	});
	execute<UserConfig>('user_config').then((userConfig) => {
		set(userConfig);
	});

	const setDbeaverPath = async (path: string) => {
		const config = await execute<UserConfig>('set_dbeaver_path', { dbeaverPath: path }, true);
		set(config);
	};

	const login = async (profile: string) => {
		const config = await execute<UserConfig>('login', { profile });
		set(config);
		loggedIn.set(true);
	};

	const favoriteTrackedName = async (name: string) => {
		const config = await execute<UserConfig>('favorite', {
			name
		});
		set(config);
		homeStore.refresh(false);
	};

	const savePrefferedEnvs = async (envs: AwsEnv[]) => {
		const config = await execute<UserConfig>('save_preffered_envs', {
			envs
		});
		console.log('new config', config);
		set(config);
	};

	return { subscribe, login, setDbeaverPath, favoriteTrackedName, savePrefferedEnvs };
};
export const userStore = createUserStore();
