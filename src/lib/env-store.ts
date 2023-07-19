import { writable } from 'svelte/store';
import { AwsEnv } from './types';
const envImportance = {
	[AwsEnv.DEVNULL]: 0,
	[AwsEnv.PLAY]: 1,
	[AwsEnv.LAB]: 2,
	[AwsEnv.DEV]: 3,
	[AwsEnv.DEMO]: 4,
	[AwsEnv.PROD]: 5
};
export const ENVIRONMENTS = Object.values(AwsEnv).sort(
	(a, b) => envImportance[a] - envImportance[b]
);
export const envStore = writable<AwsEnv>(AwsEnv.DEV);
