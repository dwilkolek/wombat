import { get, writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv, RdsInstance } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';

const createDbStore = () => {
	const innerStore = writable(new Map<AwsEnv, RdsInstance[]>());
	const getDatabases = async (env: AwsEnv): Promise<RdsInstance[]> => {
		if (get(innerStore).has(env)) {
			return get(innerStore).get(env)!;
		} else {
			const databases = await invoke<RdsInstance[]>('databases', { env });
			const clone = new Map(get(innerStore));
			clone.set(env, databases);
			innerStore.set(clone);
			return databases;
		}
	};
	return { ...innerStore, getDatabases };
};

listen('cache-refreshed', () => {
	dbStore.set(new Map());
});
listen('logged-out', () => {
	dbStore.set(new Map());
});
export const dbStore = createDbStore();
