import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';

export type DeploymentRolloutStatus = {
	deployment_id: string;
	service_name: string;
	cluster_arn: string;
	rollout_status: 'Unknown' | 'Completed' | 'Failed' | 'In Progress';
};
const createDeploymentStore = () => {
	const deplyoments = writable<DeploymentRolloutStatus[]>([]);
	listen<DeploymentRolloutStatus>('deployment', (event) => {
		console.log('deployment update: ', event);
		deplyoments.update((deployments) => {
			return [
				...deployments.filter(
					(deployment) => deployment.deployment_id != event.payload.deployment_id
				),
				event.payload
			];
		});
	});

	return {
		...deplyoments,
		clear: (deploymentId: string) => {
			deplyoments.update((deplyoments) => {
				return deplyoments.filter((d) => d.deployment_id != deploymentId);
			});
		}
	};
};
export const deplyomentStore = createDeploymentStore();
