import { writable } from 'svelte/store';
import { execute } from './error-store';
import type { AwsEnv, UserConfig, WombatAwsProfile } from '../types';
import { emit, listen } from '@tauri-apps/api/event';
import type { ProxyEventMessage } from './task-store';
import { invoke } from '@tauri-apps/api';

const createUserStore = () => {
	const loggedIn = writable(false);
	const { subscribe, set, update } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		known_profiles: [],
		last_used_profile: undefined,
		logs_dir: '',
		db_proxy_port_map: {},
		service_proxy_port_map: {},
		preferences: {}
	});
	execute<UserConfig>('user_config').then((config) => {
		console.log('user_config', config);
		set({ ...config });
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
		// DO NOT FUCKING REMOVE. PREVENTS FROM PROXY HANGING
		invoke('ping');
	});

	const setDbeaverPath = async (path: string) => {
		const config = await execute<UserConfig>('set_dbeaver_path', { dbeaverPath: path }, true);
		set(prepareConfig(config));
	};

	const setLogsDir = async (path: string) => {
		const config = await execute<UserConfig>('set_logs_dir_path', { logsDir: path }, true);
		set(prepareConfig(config));
	};

	const login = async (profile: WombatAwsProfile | undefined) => {
		if (!profile) {
			return;
		}
		const config = await execute<UserConfig>('login', { profile: profile.name });
		set(prepareConfig(config));
		loggedIn.set(true);
		console.log('setting', profile);
		emit('logged-in');
	};

	const favoriteTrackedName = async (name: string) => {
		const config = await execute<UserConfig>('favorite', {
			name
		});
		set(prepareConfig(config));
	};

	const savePrefferedEnvs = async (envs: AwsEnv[]) => {
		const config = await execute<UserConfig>('save_preffered_envs', {
			envs
		});

		set(prepareConfig(config));
	};

	return {
		subscribe,
		login,
		setDbeaverPath,
		setLogsDir,
		favoriteTrackedName,
		savePrefferedEnvs
	};
};
const prepareConfig= (config: UserConfig) => {
	for (let preference of Object.values(config.preferences)) {
		preference.tracked_names.sort((a, b) => a.localeCompare(b))
	}
	return config
}
export const userStore = createUserStore();
