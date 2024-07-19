import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const features = writable({
		devWay: false,
		restartEcsService: false,
		proxyCustomHeaders: false,
		lambdaApps: false
	});

	Promise.all([
		invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
		invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' }),
		invoke<boolean>('is_feature_enabled', { feature: 'proxy-custom-headers' }),
		invoke<boolean>('is_feature_enabled', { feature: 'lambda-apps' })
	]).then(([devWay, restartEcsService, proxyCustomHeaders, lambdaApps]) => {
		features.set({
			devWay,
			restartEcsService,
			proxyCustomHeaders,
			lambdaApps
		});
	});
	listen('cache-refreshed', () => {
		Promise.all([
			invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
			invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' }),
			invoke<boolean>('is_feature_enabled', { feature: 'proxy-custom-headers' }),
			invoke<boolean>('is_feature_enabled', { feature: 'lambda-apps' })
		]).then(([devWay, restartEcsService, proxyCustomHeaders, lambdaApps]) => {
			features.set({
				devWay,
				restartEcsService,
				proxyCustomHeaders,
				lambdaApps
			});
		});
	});
	return features;
};

export const featuresStore = createFeatureStore();
