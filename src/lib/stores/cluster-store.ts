import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { get, writable } from 'svelte/store';
import { AwsEnv, type Cluster } from '../types';
export const envImportance = {
	[AwsEnv.DEVNULL]: 0,
	[AwsEnv.PLAY]: 1,
	[AwsEnv.LAB]: 2,
	[AwsEnv.DEV]: 3,
	[AwsEnv.DEMO]: 4,
	[AwsEnv.PROD]: 5
};
const clusterStoreCreate = () => {
	const activeCluser = writable<Cluster>();
	const clusters = writable<Cluster[]>([]);
	invoke<Cluster[]>('clusters').then((resp) => {
		const clusterLists = [];
		clusterLists.push(...resp.sort((a, b) => envImportance[a.env] - envImportance[b.env]));
		const dev_env = clusterLists.find((it) => it.env == AwsEnv.DEV);
		if (dev_env) {
			activeCluser.set(dev_env);
		}
		clusters.set(clusterLists);
	});
	listen('cache-refreshed', () => {
		refresh();
	});
	const refresh = () => {
		invoke<Cluster[]>('clusters').then(async (resp) => {
			const clusterLists: Cluster[] = [];
			clusterLists.push(...resp.sort((a, b) => envImportance[a.env] - envImportance[b.env]));
			clusters.set(clusterLists);
			const currentActiveCluster = get(activeCluser);
			if (!clusterLists.includes(currentActiveCluster)) {
				for (const cluster of clusterLists) {
					if (cluster.env == AwsEnv.DEV) {
						activeCluser.set(cluster);
					}
				}
			}
		});
	};

	return { clusters, activeCluser, refresh };
};
listen('logged-out', () => {
	clusterStore.clusters.set([]);
});
listen('logged-in', () => {
	clusterStore.refresh();
});
export const clusterStore = clusterStoreCreate();
