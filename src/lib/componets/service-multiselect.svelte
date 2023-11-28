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


<div class={`w-full flex flex-col items-center mx-auto group ${open ? 'is-open' : ''}`}>
	<div class="w-full">
		<div class="flex flex-col items-center relative">
			<div class="w-full">
				<div class="p-1 flex border border-base-content/[.2] rounded-lg">
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
											class="feather feather-x cursor-pointer hover:text-teal-400 rounded-full w-4 h-4 ml-2"
										>
											<line x1="18" y1="6" x2="6" y2="18"></line>
											<line x1="6" y1="6" x2="18" y2="18"></line>
										</svg>
									</div>
								</div>
							</div>
						{/each}
						<div class="flex-1">
							<input
								placeholder=""
								class="bg-transparent p-1 px-2 appearance-none outline-none h-full w-full text-gray-800"
							/>
						</div>
					</div>
					<div class="w-8 py-1 pl-2 pr-1 border-l flex items-center border-neutral">
						<button
							class={`cursor-pointer w-6 h-6 outline-none focus:outline-none ${
								open ? '' : 'rotate-180'
							}`}
							on:click={() => (open = !open)}
						>
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
								class="feather feather-chevron-up w-4 h-4"
							>
								<polyline points="18 15 12 9 6 15"></polyline>
							</svg>
						</button>
					</div>
				</div>
			</div>
			{#if open}
				<div
					class="absolute shadow top-100 bg-base-200 w-full left-0 rounded max-h-select overflow-y-auto z-50"
				>
					<div class="flex flex-col w-full base-300">
						{#await services then services}
							{#each services as services, i}
								<!-- {#if services.length > 0}
									<div class="w-full rounded-t p-2 text-xs pl-4">
										{#if i == 0}
											-- favorite --
										{/if}
										{#if i == 1}
											-- rest --
										{/if}
									</div>
								{/if} -->
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
												<div class="mx-2 leading-6 text-sm"> {#if i == 0} * {/if} {o.name}</div>
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
	<div class="w-screen h-screen fixed" on:click={() => open = !open}></div>
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
