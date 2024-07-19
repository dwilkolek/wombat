import { derived, writable } from 'svelte/store';
import { execute } from './error-store';
import type { AwsEnv, UserConfig, WombatAwsProfile } from '../types';
import { emit } from '@tauri-apps/api/event';

const createUserStore = () => {
	const loggedIn = writable(false);
	const { subscribe, set } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		known_profiles: [],
		last_used_profile: undefined,
		logs_dir: '',
		db_proxy_port_map: {},
		service_proxy_port_map: {},
		lambda_app_proxy_port_map: {},
		preferences: {}
	});
	execute<UserConfig>('user_config').then((config) => {
		set(prepareConfig(config));
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
const prepareConfig = (config: UserConfig) => {
	for (const preference of Object.values(config.preferences)) {
		preference.tracked_names.sort((a, b) => a.localeCompare(b));
	}
	return config;
};
export const userStore = createUserStore();

export const activeProfilePreferences = derived([userStore], (stores) => {
	return (
		stores[0].preferences[stores[0].last_used_profile ?? ''] ?? {
			preffered_environments: [],
			tracked_names: []
		}
	);
});
