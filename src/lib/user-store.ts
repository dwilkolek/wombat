import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';
import type { UserConfig } from '../routes/types';
import { version } from '$app/environment';
declare const gtag: any;
const createUserStore = () => {
	const { subscribe, set } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		favourite_db_arns: [],
		favourite_service_names: [],
		known_profiles: [],
		last_used_profile: undefined
	});
	invoke<UserConfig>('user_config').then((userConfig) => {
		set(userConfig);
	});

	subscribe((userConfig) => {
		console.log('new! user_config', userConfig);
	});
	const setDbeaverPath = async (path: string) => {
		try {
			const config = await invoke<UserConfig>('set_dbeaver_path', { dbeaverPath: path });
			set(config);
		} catch (e) {
			console.error('Whoops', e);
		}
	};

	const login = async (profile: string) => {
		try {
			const config = await invoke<UserConfig>('login', { profile });
			set(config);
		} catch (e) {
			console.error('Whoops', e);
		}
	};

	const favoriteService = async (service_name: string) => {
		try {
			const config = await invoke<UserConfig>('toggle_service_favourite', {
				serviceName: service_name
			});
			set(config);
		} catch (e) {
			console.error('Whoops', e);
		}
	};

	const favoriteDb = async (db_arn: string) => {
		try {
			const config = await invoke<UserConfig>('toggle_db_favourite', { dbArn: db_arn });
			set(config);
		} catch (e) {
			console.error('Whoops', e);
		}
	};

	return { subscribe, login, setDbeaverPath, favoriteService, favoriteDb };
};
export const userStore = createUserStore();
