<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type { ServiceDetails } from '$lib/types';
	import { removeEcsTaskDefinitionsReason } from '$lib/stores/reasons';
	import { ask, message } from '@tauri-apps/plugin-dialog';

	interface Props {
		service: ServiceDetails;
	}

	let { service }: Props = $props();

	let disabledReason = removeEcsTaskDefinitionsReason(service);

	let cleanupStarted = $state(false);
</script>

<span
	class="tooltip tooltip-left flex"
	data-tip={$disabledReason ?? 'Remove non-platform task definitions ' + service.td_family}
>
	<button
		aria-label="Remove non-platform task definitions"
		disabled={!!$disabledReason || cleanupStarted}
		class={$disabledReason ? 'opacity-30' : ''}
		onclick={async (e) => {
			e.preventDefault();
			cleanupStarted = true;
			const removed = await invoke<string[]>('remove_task_definitions', {
				service,
				dryRun: true
			});
			let response = await ask(
				`Planning to remove ${removed.length} task defintions. First few:\n` +
					removed
						.slice(0, 35)
						.map((td) => td.split('/').at(-1))
						.join(', '),
				{
					title: 'Task definition removal - dry run',
					okLabel: 'Proceed',
					cancelLabel: 'Abort',
					kind: 'warning'
				}
			);
			if (response) {
				const removedEffectively = await invoke<string[]>('remove_task_definitions', {
					service,
					dryRun: false
				});
				const notRemoved = removed.filter((r) => !removedEffectively.includes(r));
				let messageStr = `Removed ${removedEffectively.length}. `;
				if (notRemoved.length > 0) {
					messageStr +=
						`Failed to remove: \n` +
						notRemoved
							.slice(0, 35)
							.map((td) => td.split('/').at(-1))
							.join(', ');
				}
				await message(messageStr);
			}
			cleanupStarted = false;
			console.log('removed', removed);
		}}
	>
		{#if cleanupStarted}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-4 h-4"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
				/>
			</svg>
		{:else}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				viewBox="0 0 20 20"
				fill="currentColor"
				class="size-4"
			>
				<path
					fill-rule="evenodd"
					d="M14.5 10a4.5 4.5 0 0 0 4.284-5.882c-.105-.324-.51-.391-.752-.15L15.34 6.66a.454.454 0 0 1-.493.11 3.01 3.01 0 0 1-1.618-1.616.455.455 0 0 1 .11-.494l2.694-2.692c.24-.241.174-.647-.15-.752a4.5 4.5 0 0 0-5.873 4.575c.055.873-.128 1.808-.8 2.368l-7.23 6.024a2.724 2.724 0 1 0 3.837 3.837l6.024-7.23c.56-.672 1.495-.855 2.368-.8.096.007.193.01.291.01ZM5 16a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"
					clip-rule="evenodd"
				/>
				<path
					d="M14.5 11.5c.173 0 .345-.007.514-.022l3.754 3.754a2.5 2.5 0 0 1-3.536 3.536l-4.41-4.41 2.172-2.607c.052-.063.147-.138.342-.196.202-.06.469-.087.777-.067.128.008.257.012.387.012ZM6 4.586l2.33 2.33a.452.452 0 0 1-.08.09L6.8 8.214 4.586 6H3.309a.5.5 0 0 1-.447-.276l-1.7-3.402a.5.5 0 0 1 .093-.577l.49-.49a.5.5 0 0 1 .577-.094l3.402 1.7A.5.5 0 0 1 6 3.31v1.277Z"
				/>
			</svg>
		{/if}
	</button>
</span>
