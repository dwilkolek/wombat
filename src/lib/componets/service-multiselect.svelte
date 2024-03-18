<script lang="ts">
	import { clusterStore } from '$lib/stores/cluster-store';
	import { userStore } from '$lib/stores/user-store';
	import { serviceStore } from '$lib/stores/service-store';
	import type { EcsService } from '$lib/types';

	let open = false;
	$: activeCluser = clusterStore.activeCluser;
	$: tracked_names = $userStore.tracked_names;
	$: selectedServices = serviceStore.selectedServices;
	$: services = serviceStore.getServices($activeCluser).then((services) => {
		return services
			.filter((a) => a.name.includes(inputValue))
			.toSorted((a, b) => {
				const aT = tracked_names.includes(a.name) ? 1 : 0;
				const bT = tracked_names.includes(b.name) ? 1 : 0;
				return bT - aT;
			});
	});
	const toggle = () => {
		open = !open;
		if (!open) {
			inputValue = '';
		}
	};

	let inputValue = '';
	let inputElement: HTMLElement;
	$: if (inputElement) {
		setTimeout(() => {
			inputElement.focus();
		});
	}

	$: select = (app: EcsService) => {
		serviceStore.selectService(app);
	};
</script>

<div class={`w-full flex flex-col items-center mx-auto`}>
	<div class="w-full">
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div class="flex flex-col items-center relative cursor-pointer">
			<div class="w-full">
				<div
					class="h-8 flex flex-row justify-center items-center p-1 form-control border border-radius-sm border-slate select-bordered rounded-btn"
				>
					<div class="flex flex-auto flex-wrap items-center gap-1 shrink">
						{#each $selectedServices as s}
							<div class="badge badge-info text-xs">
								{s.name}
								<button on:click={() => select(s)}>
									<svg
										xmlns="http://www.w3.org/2000/svg"
										fill="none"
										viewBox="0 0 24 24"
										class="inline-block w-4 h-4 stroke-current"
										><path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M6 18L18 6M6 6l12 12"
										></path></svg
									>
								</button>
							</div>
						{/each}
						<div class="grow h-full" on:pointerdown={toggle}>&nbsp;</div>
					</div>

					<button class={`z-50 outline-none focus:outline-none ml-2`} on:click={toggle}>
						{#if open}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 20 20"
								fill="currentColor"
								class="w-5 h-5"
							>
								<path
									fill-rule="evenodd"
									d="M14.77 12.79a.75.75 0 01-1.06-.02L10 8.832 6.29 12.77a.75.75 0 11-1.08-1.04l4.25-4.5a.75.75 0 011.08 0l4.25 4.5a.75.75 0 01-.02 1.06z"
									clip-rule="evenodd"
								/>
							</svg>
						{:else}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 20 20"
								fill="currentColor"
								class="w-5 h-5"
							>
								<path
									fill-rule="evenodd"
									d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
									clip-rule="evenodd"
								/>
							</svg>
						{/if}
					</button>
				</div>
			</div>
			{#if open}
				<div
					class="absolute shadow top-full hh
					bg-base-200 w-full left-0 rounded overflow-y-auto z-50"
				>
					<div class="flex flex-col w-full base-300">
						<div class="m-2">
							<input
								autocomplete="off"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
								type="text"
								placeholder="Search"
								class="input input-sm input-bordered input-accent w-full"
								bind:value={inputValue}
								bind:this={inputElement}
							/>
						</div>
						<div class="overflow-auto max-h-[250px]">
							{#await services then services}
								{#each services as o}
									<!-- svelte-ignore a11y-click-events-have-key-events -->
									<!-- svelte-ignore a11y-no-static-element-interactions -->
									<div
										class={`cursor-pointer w-full hover:bg-base-300`}
										on:click={() => {
											select(o);
											inputValue = '';
										}}
									>
										<div
											class="flex w-full items-center p-1 border-transparent border-l-2 relative"
										>
											<div class="w-full items-center flex">
												<div class={`mx-2 leading-6 flex items-center gap-1`}>
													{#if $selectedServices.some((selected) => selected.name == o.name)}
														<svg
															xmlns="http://www.w3.org/2000/svg"
															viewBox="0 0 20 20"
															fill="currentColor"
															class="w-4 h-4 text-lime-400"
														>
															<path
																fill-rule="evenodd"
																d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z"
																clip-rule="evenodd"
															/>
														</svg>
													{:else}
														<svg
															xmlns="http://www.w3.org/2000/svg"
															viewBox="0 0 20 20"
															fill="currentColor"
															class="w-4 h-4"
														>
															<path
																fill-rule="evenodd"
																d="M16.704 4.153a.75.75 0 01.143 1.052l-8 10.5a.75.75 0 01-1.127.075l-4.5-4.5a.75.75 0 011.06-1.06l3.894 3.893 7.48-9.817a.75.75 0 011.05-.143z"
																clip-rule="evenodd"
															/>
														</svg>
													{/if}
													{#if tracked_names.includes(o.name)}
														<svg
															xmlns="http://www.w3.org/2000/svg"
															viewBox="0 0 20 20"
															fill="currentColor"
															class="w-3 h-3 text-warning"
														>
															<path
																fill-rule="evenodd"
																d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z"
																clip-rule="evenodd"
															/>
														</svg>
													{/if}
													{o.name}
												</div>
											</div>
										</div>
									</div>
								{/each}
							{/await}
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

{#if open}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div class="w-screen h-screen bottom-0 left-0 fixed bg-salte" on:click={toggle}></div>
{/if}
