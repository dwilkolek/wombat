import { derived, get, writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { Cluster, EcsService } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { clusterStore } from './cluster-store';

const createServiceStore = () => {
	const innerStore = writable(new Map<Cluster, EcsService[]>());
	const selectedService = writable<EcsService | null>(null);
	const selectedServices = writable<EcsService[]>([]);
	const getServices = async (cluster?: Cluster): Promise<EcsService[]> => {
		if (!cluster) {
			return [];
		}
		console.log('do instant', get(innerStore).has(cluster), get(innerStore));
		const store = get(innerStore);
		if (store.has(cluster)) {
			return store.get(cluster)!;
		} else {
			console.log('calling services');
			const services = await invoke<EcsService[]>('services', { cluster });
			innerStore.update((old) => {
				const copy = new Map();
				old.forEach((eValue, eKey) => {
					copy.set(eKey, eValue);
				});
				copy.set(cluster, services);
				return copy;
			});
			return services;
		}
	};

	const selectService = (selection: EcsService) => {
		selectedService.set(selection);
		selectedServices.update((services) => {
			let newSelection;
			if (services.includes(selection)) {
				newSelection = services.filter((s) => s.arn != selection.arn);
			} else {
				newSelection = [...services, selection];
			}

			return newSelection;
		});
	};

	clusterStore.activeCluser.subscribe((activeCluster) => {
		getServices(activeCluster).then((servicesInNewCluster) => {
			const service = get(selectedService);
			if (service) {
				if (service.cluster_arn != activeCluster.arn) {
					const serviceFromNewCluster = servicesInNewCluster.find((s) => s.name === service.name);
					selectedService.set(serviceFromNewCluster ?? null);
				}
			}

			const newSelectedServices = get(selectedServices)
				.map((service) => servicesInNewCluster.find((s) => s.name === service.name))
				.filter((o) => !!o) as EcsService[];
			selectedServices.set(newSelectedServices);
		});
	});
	return {
		...innerStore,
		selectedService,
		selectedServices,
		getServices,
		selectService
	};
};

listen('cache-refreshed', () => {
	serviceStore.set(new Map());
});

listen('logged-out', () => {
	serviceStore.set(new Map());
});

export const serviceStore = createServiceStore();
export const servicedForActiveCluster = derived(
	[clusterStore.activeCluser, serviceStore],
	([cluster, clusterServicesMap]) => {
		return clusterServicesMap.get(cluster) ?? [];
	}
);
