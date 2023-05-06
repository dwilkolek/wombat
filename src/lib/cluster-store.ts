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
const clusterStoreCreate = () => {
	const activeCluser = writable<Cluster>();
	let clusterLists: Cluster[] = [];
	const clusters = readable(clusterLists);
	invoke<Cluster[]>('clusters').then((resp) => {
		clusterLists.push(...resp.sort((a, b) => (envImportance[a.env] - envImportance[b.env])));
		const dev_env = clusterLists.find(it => it.env == AwsEnv.DEV)
		if (dev_env){
			activeCluser.set(dev_env)
		}
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
	
	return { clusters, activeCluser, refresh };
};

export const clusterStore = clusterStoreCreate();
