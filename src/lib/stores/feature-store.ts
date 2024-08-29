import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const features = writable({
		devWay: false,
		restartEcsService: false,
		proxyCustomHeaders: false,
		lambdaApps: false,
		prodActionsEnabled: false
	});

	Promise.all([
		invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
		invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' }),
		invoke<boolean>('is_feature_enabled', { feature: 'proxy-custom-headers' }),
		invoke<boolean>('is_feature_enabled', { feature: 'lambda-apps' }),
		invoke<boolean>('is_feature_enabled', { feature: 'prod-actions-enabled' })
	]).then(([devWay, restartEcsService, proxyCustomHeaders, lambdaApps, prodActionsEnabled]) => {
		features.set({
			devWay,
			restartEcsService,
			proxyCustomHeaders,
			lambdaApps,
			prodActionsEnabled
		});
	});
	listen('cache-refreshed', () => {
		Promise.all([
			invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
			invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' }),
			invoke<boolean>('is_feature_enabled', { feature: 'proxy-custom-headers' }),
			invoke<boolean>('is_feature_enabled', { feature: 'lambda-apps' }),
			invoke<boolean>('is_feature_enabled', { feature: 'prod-actions-enabled' })
		]).then(([devWay, restartEcsService, proxyCustomHeaders, lambdaApps, prodActionsEnabled]) => {
			features.set({
				devWay,
				restartEcsService,
				proxyCustomHeaders,
				lambdaApps,
				prodActionsEnabled
			});
		});
	});
	return features;
};

export const featuresStore = createFeatureStore();
