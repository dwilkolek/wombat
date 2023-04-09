import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { readable, writable } from 'svelte/store';
import { AwsEnv, type Cluster } from './types';

const envStoreCreate = () => {
	const currentEnv = writable<AwsEnv>();
	const activeCluser = writable<Cluster>();
	let clusterLists: Cluster[] = [];
	const clusters = readable(clusterLists);
	invoke<Cluster[]>('clusters').then((resp) => {
		clusterLists.push(...resp);
		currentEnv.set(AwsEnv.DEV);
	});
	listen('cache-refreshed', () => {
		refresh();
	});
	const refresh = () => {
		invoke<Cluster[]>('clusters').then((resp) => {
			clusterLists = [];
			clusterLists.push(...resp);
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
