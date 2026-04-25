import { derived, writable } from 'svelte/store';
import { execute } from './error-store';
import type { AwsEnv, UserConfig, WombatAwsAccount } from '../types';
import { emit } from '@tauri-apps/api/event';

const createUserStore = () => {
	const loggedIn = writable(false);
	const { subscribe, set } = writable<UserConfig>({
		id: undefined,
		dbeaver_path: undefined,
		known_profiles: [],
		last_used_profile: undefined,
		logs_dir: '',
		arn_to_proxy_port_map: {},
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

	const login = async (account: WombatAwsAccount | undefined) => {
		if (!account) {
			return;
		}
		const config = await execute<UserConfig>('login', { profile: account.id });
		set(prepareConfig(config));
		loggedIn.set(true);
		console.log('setting', account);
		emit('logged-in');
	};

	const favoriteTrackedName = async (name: string) => {
		const config = await execute<UserConfig>('favorite', {
			name
		});
		set(prepareConfig(config));
	};

	const savePreferredEnvs = async (envs: AwsEnv[]) => {
		const config = await execute<UserConfig>('save_preferred_envs', {
			envs
		});

		set(prepareConfig(config));
	};

	const logout = async () => {
		await execute<void>('logout');
		loggedIn.set(false);
	};

	return {
		subscribe,
		loggedIn: { subscribe: loggedIn.subscribe },
		login,
		logout,
		setDbeaverPath,
		setLogsDir,
		favoriteTrackedName,
		savePreferredEnvs
	};
};
const prepareConfig = (config: UserConfig) => {
	for (const preference of Object.values(config.preferences)) {
		preference.tracked_names.sort((a, b) => a.localeCompare(b));
	}
	return config;
};
export const userStore = createUserStore();

export const activeAccountPreferences = derived([userStore], (stores) => {
	return (
		stores[0].preferences[stores[0].last_used_profile ?? ''] ?? {
			preferred_environments: [],
			tracked_names: []
		}
	);
});
