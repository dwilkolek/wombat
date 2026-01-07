import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const defaultFs = {
		loading: true,

		//TODO: consider and delete
		deployEcsService: false,
		deployEcsWithTags: false,
		removeEcsTaskDefinitions: false,
		tasksPage: false,
		proxyCustomHeaders: false,
		cookieSessionProxy: false,

		restartEcsService: false,
		startEcsProxy: false,
		startRdsProxy: false,
		startLambdaProxy: false,
		getRdsSecret: false,
		lambdaApps: false,
		ecsProdActions: false,
		rdsProdActions: false,
		lambdaProdActions: false,
		rdsProdConnWrite: false,
		rdsConnWrite: false,
		debug: false
	};
	const features = writable(defaultFs);
	const isValidFeature = (v: string): v is keyof typeof defaultFs => {
		return defaultFs[v as keyof typeof defaultFs] != undefined;
	};
	async function refreshFeatures() {
		features.set(defaultFs);
		const fs = await invoke<string[]>('all_features_enabled');
		features.update((prev) => {
			const newFs = fs.reduce((acc, v) => {
				if (!isValidFeature(v)) {
					console.warn(`Unknown feature: ${v}`);
					return acc;
				}
				return { ...acc, [v]: true };
			}, prev);
			newFs.loading = false;
			return newFs;
		});
	}
	refreshFeatures();
	listen('cache-refreshed', () => {
		refreshFeatures();
	});

	return { ...features, refreshFeatures };
};

export const featuresStore = createFeatureStore();
