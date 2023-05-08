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
	const currentEnv = writable<AwsEnv>(AwsEnv.DEV);
	return { currentEnv, envs: Object.values(AwsEnv).sort((a, b) => envImportance[a] - envImportance[b]) };
};

export const envStore = envStoreCreate();
