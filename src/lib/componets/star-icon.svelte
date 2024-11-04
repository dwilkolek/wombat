<script lang="ts">
	interface Props {
		isSelected: boolean;
	}

	let { isSelected }: Props = $props();
	// export let size = '1.8em';
	let selected = '#fcd703';
	let notSelected = 'rgba(252, 215, 3, 0.5)';
	let transition = isSelected ? notSelected : selected;
	let color = $state(isSelected ? selected : notSelected);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	onmouseover={() => {
		color = transition;
	}}
	onmouseout={() => {
		color = isSelected ? selected : notSelected;
	}}
	onfocus={() => {
		color = transition;
	}}
	onblur={() => {
		color = isSelected ? selected : notSelected;
	}}
>
	<svg
		xmlns="http://www.w3.org/2000/svg"
		fill={color}
		viewBox="0 0 24 24"
		stroke-width="1.5"
		stroke={color}
		class="w-6 h-6"
	>
		<path
			stroke-linecap="round"
			stroke-linejoin="round"
			d="M11.48 3.499a.562.562 0 011.04 0l2.125 5.111a.563.563 0 00.475.345l5.518.442c.499.04.701.663.321.988l-4.204 3.602a.563.563 0 00-.182.557l1.285 5.385a.562.562 0 01-.84.61l-4.725-2.885a.563.563 0 00-.586 0L6.982 20.54a.562.562 0 01-.84-.61l1.285-5.386a.562.562 0 00-.182-.557l-4.204-3.602a.563.563 0 01.321-.988l5.518-.442a.563.563 0 00.475-.345L11.48 3.5z"
		/>
	</svg>

	<!-- <Icon data={star} {size} fill={color} stroke={color} /> -->
</div>
