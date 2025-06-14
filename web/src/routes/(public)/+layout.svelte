<script lang="ts">
	import '../../app.css';
</script>

<div class="min-h-screen bg-gradient-to-br from-background to-muted">
	<div class="flex min-h-screen items-center justify-center p-4">
		<div class="w-full max-w-md">
			<div class="mb-8 text-center">
				<h1 class="text-3xl font-bold text-foreground">Clio</h1>
				<p class="mt-2 text-muted-foreground">Enterprise Search Platform</p>
			</div>
			<slot />
		</div>
	</div>
</div>