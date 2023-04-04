import { invoke } from '@tauri-apps/api';
import { readable, writable } from 'svelte/store';
import { Env, type Cluster, type UserConfig } from './types';

const envStoreCreate = () => {
	const currentEnv = writable<Env>();
	const activeCluser = writable<Cluster>();
	const clusterLists: Cluster[] = [];
	const clusters = readable(clusterLists);
	invoke<Cluster[]>('clusters').then((resp) => {
		clusterLists.push(...resp);
		currentEnv.set(Env.DEV);
	});
	currentEnv.subscribe((env) => {
		activeCluser.set(clusterLists.find((cluster) => cluster.env == env)!!);
	});
	return { clusters, activeCluser, currentEnv };
};

export const envStore = envStoreCreate();
