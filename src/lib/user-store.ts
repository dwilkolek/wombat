import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';
import { execute } from './error-store';
import type { UserConfig } from './types';

const createUserStore = () => {
	const { subscribe, set } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		favourite_names: [],
		known_profiles: [],
		last_used_profile: undefined
	});
	execute<UserConfig>('user_config').then((userConfig) => {
		set(userConfig);
	});

	subscribe((userConfig) => {
		console.log('new! user_config', userConfig);
	});
	const setDbeaverPath = async (path: string) => {
		const config = await execute<UserConfig>('set_dbeaver_path', { dbeaverPath: path });
		set(config);
	};

	const login = async (profile: string) => {
		const config = await execute<UserConfig>('login', { profile });
		set(config);
	};

	const favoriteToggle = async (name: string) => {
		const config = await execute<UserConfig>('toggle_favourite', {
			name: name
		});
		set(config);
	};

	return { subscribe, login, setDbeaverPath, favoriteToggle };
};
export const userStore = createUserStore();
