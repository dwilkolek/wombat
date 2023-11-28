<script>
	import { clusterStore } from '$lib/stores/cluster-store';
	import { userStore } from '$lib/stores/user-store';
	import { serviceStore } from '$lib/stores/service-store';
	let open = false;

	$: activeCluser = clusterStore.activeCluser;
	$: tracked_names = $userStore.tracked_names;
	$: selectedServices = serviceStore.selectedServices;
	$: selectedArns = $selectedServices.map((s) => s.arn);
	$: services = serviceStore.getServices($activeCluser).then((services) => [
		services.filter((a) => {
			return tracked_names.includes(a.name) && !selectedArns.includes(a.arn);
		}),
		services.filter((a) => {
			return !tracked_names.includes(a.name) && !selectedArns.includes(a.arn);
		})
	]);

	$: select = serviceStore.selectService;
</script>

<div class={`w-full flex flex-col items-center mx-auto`}>
	<div class="w-full">
		<div class="flex flex-col items-center relative">
			<div class="w-full ">
				<div
					class="p-1.5 min-h-12 flex flex-row form-control border border-radius-sm border-slate select-bordered rounded-btn"
				>
					<div class="flex flex-auto flex-wrap">
						{#each $selectedServices as s}
							<div
								class="flex justify-center items-center m-1 font-medium py-1 px-2 rounded-full bg-success text-black border border-neutral"
							>
								<div class="text-xs font-normal leading-none max-w-full flex-initial">
									{s.name}
								</div>
								<div class="flex flex-auto flex-row-reverse">
									<!-- svelte-ignore a11y-click-events-have-key-events -->
									<!-- svelte-ignore a11y-no-static-element-interactions -->
									<div on:click={() => select(s)}>
										<svg
											xmlns="http://www.w3.org/2000/svg"
											width="100%"
											height="100%"
											fill="none"
											viewBox="0 0 24 24"
											stroke="currentColor"
											stroke-width="2"
											stroke-linecap="round"
											stroke-linejoin="round"
											class="feather feather-x cursor-pointer hover:text-black rounded-full w-4 h-4 ml-2"
										>
											<line x1="18" y1="6" x2="6" y2="18"></line>
											<line x1="6" y1="6" x2="18" y2="18"></line>
										</svg>
									</div>
								</div>
							</div>
						{/each}
					</div>

					<button class={`outline-none focus:outline-none`} on:click={() => (open = !open)}>
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
					class="absolute shadow top-100 bg-base-200 w-full left-0 rounded max-h-select overflow-y-auto z-50"
				>
					<div class="flex flex-col w-full base-300">
						{#await services then services}
							{#each services as services, i}
								{#each services as o}
									<!-- svelte-ignore a11y-click-events-have-key-events -->
									<!-- svelte-ignore a11y-no-static-element-interactions -->
									<div
										class="cursor-pointer w-full rounded-t hover:bg-base-300"
										on:click={() => select(o)}
									>
										<div
											class="flex w-full items-center p-1 border-transparent border-l-2 relative"
										>
											<div class="w-full items-center flex">
												<div class="mx-2 leading-6 text-sm flex items-center gap-1">
													{#if i == 0}
														<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3 h-3 text-warning">
															<path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" />
														</svg>
													{/if}
													{o.name}
												</div>
											</div>
										</div>
									</div>
								{/each}
							{/each}
						{/await}
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

{#if open}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<div
		class="w-screen h-screen bottom-0 left-0 fixed bg-salte"
		on:click={() => (open = !open)}
	></div>
{/if}

<style>
	.top-100 {
		top: 100%;
	}
	.bottom-100 {
		bottom: 100%;
	}
	.max-h-select {
		max-height: 300px;
	}
</style>
