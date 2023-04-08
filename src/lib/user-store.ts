import { invoke } from '@tauri-apps/api';
import { readable, writable } from 'svelte/store';
import { execute } from './error-store';
import type { UserConfig } from './types';
import { homeStore } from './home-store';

const createUserStore = () => {
	const loggedIn = writable(false);
	const { subscribe, set } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		ecs: [],
		rds: [],
		known_profiles: [],
		last_used_profile: undefined
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

	const favoriteEcs = async (arn: string) => {
		const config = await execute<UserConfig>('favorite_ecs', {
			arn
		});
		set(config);
		homeStore.refresh(false);
	};
	const favoriteRds = async (arn: string) => {
		const config = await execute<UserConfig>('favorite_rds', {
			arn
		});
		set(config);
		homeStore.refresh(false);
	};

	return { subscribe, login, setDbeaverPath, favoriteEcs, favoriteRds };
};
export const userStore = createUserStore();
