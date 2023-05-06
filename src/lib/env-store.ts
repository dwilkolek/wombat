import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { readable, writable } from 'svelte/store';
import { AwsEnv, type Cluster } from './types';
const envImportance = {
	[AwsEnv.DEVNULL]: 0,
	[AwsEnv.PLAY]: 1,
	[AwsEnv.LAB]: 2,
	[AwsEnv.DEV]: 3,
	[AwsEnv.DEMO]: 4,
	[AwsEnv.PROD]: 5,
}
const envStoreCreate = () => {
	const currentEnv = writable<AwsEnv>();
	const activeCluser = writable<Cluster>();
	let clusterLists: Cluster[] = [];
	const clusters = readable(clusterLists);
	invoke<Cluster[]>('clusters').then((resp) => {
		clusterLists.push(...resp.sort((a, b) => (envImportance[a.env] - envImportance[b.env])));
		currentEnv.set(AwsEnv.DEV);
	});
	listen('cache-refreshed', () => {
		refresh();
	});
	const refresh = () => {
		invoke<Cluster[]>('clusters').then((resp) => {
			clusterLists = [];
			clusterLists.push(...resp.sort((a, b) => (envImportance[a.env] - envImportance[b.env])));
		});
	};
	currentEnv.subscribe((env) => {
		const active = clusterLists.find((cluster) => cluster.env == env);
		if (active) {
			activeCluser.set(active);
		}
	});
	return { clusters, activeCluser, currentEnv, refresh };
};

export const envStore = envStoreCreate();
