import { writable } from 'svelte/store';
import { execute } from './error-store';
import type { AwsEnv, UserConfig } from '../types';
import { emit, listen } from '@tauri-apps/api/event';
import type { ProxyEventMessage } from './task-store';

const createUserStore = () => {
	const loggedIn = writable(false);
	const { subscribe, set, update } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		tracked_names: [],
		known_profiles: [],
		last_used_profile: undefined,
		preffered_environments: [],
		logs_dir: '',
		last_selected_apps: [],
		db_proxy_port_map: {},
		service_proxy_port_map: {}
	});
	execute<UserConfig>('user_config').then((config) => {
		console.log('user_config', config);
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
	});
	listen<ProxyEventMessage>('proxy-start', (event) => {
		if (event.payload.proxy_type == 'ECS') {
			update((config) => {
				const clone = { ...config };
				if (!clone.service_proxy_port_map[event.payload.name]) {
					clone.service_proxy_port_map[event.payload.name] = {};
				}
				clone.service_proxy_port_map[event.payload.name][event.payload.env] = event.payload.port;
				return clone;
			});
		}
		if (event.payload.proxy_type == 'RDS') {
			update((config) => {
				const clone = { ...config };
				if (!clone.db_proxy_port_map[event.payload.name]) {
					clone.db_proxy_port_map[event.payload.name] = {};
				}
				clone.db_proxy_port_map[event.payload.name][event.payload.env] = event.payload.port;
				return clone;
			});
		}
	});

	const setDbeaverPath = async (path: string) => {
		const config = await execute<UserConfig>('set_dbeaver_path', { dbeaverPath: path }, true);
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
	};

	const setLogsDir = async (path: string) => {
		const config = await execute<UserConfig>('set_logs_dir_path', { logsDir: path }, true);
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
	};

	const setLastSelectedApps = async (apps: string[]) => {
		const config = await execute<UserConfig>('set_last_selected_apps', { apps }, false);
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
	};

	const login = async (profile: string) => {
		const config = await execute<UserConfig>('login', { profile });
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
		loggedIn.set(true);
		emit('logged-in');
	};

	const favoriteTrackedName = async (name: string) => {
		const config = await execute<UserConfig>('favorite', {
			name
		});
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
	};

	const savePrefferedEnvs = async (envs: AwsEnv[]) => {
		const config = await execute<UserConfig>('save_preffered_envs', {
			envs
		});
		set({ ...config, tracked_names: config.tracked_names.sort((a, b) => a.localeCompare(b)) });
	};

	return {
		subscribe,
		login,
		setDbeaverPath,
		setLogsDir,
		setLastSelectedApps,
		favoriteTrackedName,
		savePrefferedEnvs
	};
};
export const userStore = createUserStore();
