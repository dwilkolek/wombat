import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';
import { AwsEnv, type Cluster } from '../types';
const envImportance = {
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
		console.log('clusters', resp);
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
			activeCluser.subscribe((c) => {
				if (!clusterLists.includes(c)) {
					for (const cluster of clusterLists) {
						if (cluster.env == AwsEnv.DEV) {
							activeCluser.set(cluster);
						}
					}
				}
			});
		});
	};

	return { clusters, activeCluser, refresh };
};

export const clusterStore = clusterStoreCreate();
