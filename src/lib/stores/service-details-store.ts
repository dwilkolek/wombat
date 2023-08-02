import { derived, get, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv, DbInstance, ServiceDetails } from '$lib/types';
import { ENVIRONMENTS } from './env-store';
type ServiceDetailsPayload = {
	app: string;
	dbs: DbInstance[];
	services: ServiceDetails[];
};

const createServiceDetailsStore = () => {
	const innerStore = writable<ServiceDetailsPayload[]>([]);

	return { ...innerStore };
};

listen<ServiceDetailsPayload>('new-service-details', (data) => {
	setTimeout(() => {
		delete to[data.payload.app];
	}, 200);
	serviceDetailsStore.update((old) => {
		return [...old.filter((o) => o.app !== data.payload['app']), { ...data.payload }];
	});
});
listen('cache-refreshed', () => {
	serviceDetailsStore.set([]);
});
listen('logged-out', () => {
	serviceDetailsStore.set([]);
});
const serviceDetailsStore = createServiceDetailsStore();
setInterval(
	() => {
		const apps = get(serviceDetailsStore);
		apps.forEach((app) => {
			invoke('service_details', { app: app.app });
		});
	},
	5 * 60 * 1000
);
const to: { [key: string]: Promise<void> } = {};
export const serviceDetailStore = (app: string) =>
	derived([serviceDetailsStore], (stores) => {
		const apps = stores[0];
		const details = apps.find((a) => a.app === app);
		if (!details) {
			if (!to[app]) {
				to[app] = invoke('service_details', { app });
			}
			return undefined;
		}
		if (details) {
			const result = {
				app,
				envs: new Map<AwsEnv, { services: ServiceDetails[]; dbs: DbInstance[] }>()
			};
			for (const env of ENVIRONMENTS) {
				const services = details.services.filter((s) => s.env === env);
				const dbs = details.dbs.filter((s) => s.env === env);
				if (services.length || dbs.length) {
					result.envs.set(env, {
						services,
						dbs
					});
				}
			}

			return result;
		}
	});
