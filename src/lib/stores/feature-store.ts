import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const featureMap: { [key: string]: string } = {
	'dev-way': 'devWay',
	'restart-ecs-service': 'restartEcsService',
	'deploy-ecs-service': 'deployEcsService',
	'deploy-ecs-with-tags': 'deployEcsWithTags',
	'start-ecs-proxy': 'startEcsProxy',
	'start-rds-proxy': 'startRdsProxy',
	'start-lambda-proxy': 'startLambdaProxy',
	'get-rds-secret': 'getRdsSecret',
	'proxy-custom-headers': 'proxyCustomHeaders',
	'lambda-apps': 'lambdaApps',
	'ecs-prod-actions': 'ecsProdActions',
	'rds-prod-actions': 'rdsProdActions',
	'lambda-prod-actions': 'lambdaProdActions',
	'tasks-page': 'tasksPage',
	'user-session-proxy': 'userSessionProxy'
};

const createFeatureStore = () => {
	const defaultFs = {
		loading: true,
		devWay: false,
		restartEcsService: false,
		deployEcsService: false,
		deployEcsWithTags: false,
		startEcsProxy: false,
		startRdsProxy: false,
		startLambdaProxy: false,
		getRdsSecret: false,
		proxyCustomHeaders: false,
		lambdaApps: false,
		ecsProdActions: false,
		rdsProdActions: false,
		lambdaProdActions: false,
		tasksPage: false,
		userSessionProxy: false
	};
	const features = writable(defaultFs);

	async function refreshFeatures() {
		features.set(defaultFs);
		const fs = await invoke<string[]>('all_features_enabled');
		features.update((prev) => {
			const newFs = {
				...prev,
				loading: false,
				...fs.reduce((acc, v) => {
					if (!featureMap[v]) {
						console.warn(`Unknown feature: ${v}`);
						return acc;
					}
					return { ...acc, [featureMap[v] ?? v]: true };
				}, {})
			};
			console.log(newFs);
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
