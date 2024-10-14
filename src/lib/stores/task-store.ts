import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { ProxyAuthConfig } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';

type TaskKilled = {
	arn: string;
};

export enum TaskStatus {
	STARTING,
	RUNNING,
	FAILED
}
export type Task = {
	arn: string;
	name: string;
	status: TaskStatus;
	port?: number;
	proxyAuthConfig?: ProxyAuthConfig;
};

export type NewTaskParams = {
	port: number;
	proxyAuthConfig?: ProxyAuthConfig;
};

type TaskDef = { name: string; arn: string; proxyAuthConfig?: ProxyAuthConfig };

const createTaskStore = () => {
	const tasks = writable<Task[]>([]);

	const updateToStatus = (newTask: Task) => {
		tasks.update((tasks) => {
			return [...tasks.filter((t) => t.arn !== newTask.arn), newTask];
		});
	};

	const startTask = async (
		{ name, arn, proxyAuthConfig }: TaskDef,
		startTask: () => Promise<NewTaskParams>
	) => {
		updateToStatus({ arn, name, status: TaskStatus.STARTING, proxyAuthConfig });
		try {
			const { port, proxyAuthConfig } = await startTask();
			updateToStatus({
				arn,
				name,
				status: TaskStatus.RUNNING,
				port,
				proxyAuthConfig
			});
		} catch (e) {
			console.warn('Failed to start task', e);
			updateToStatus({ arn, name, status: TaskStatus.FAILED });
		}
	};

	listen<TaskKilled>('task-killed', (event) => {
		console.log('task-killed', event);
		tasks.update((tasks) => {
			return tasks.filter((t) => t.arn != event.payload.arn);
		});
	});

	const stopTask = async (arn: string) => {
		return invoke('stop_job', { arn });
	};
	return { subscribe: tasks.subscribe, startTask, stopTask };
};
export const taskStore = createTaskStore();
