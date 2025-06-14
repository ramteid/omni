<script lang="ts">
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	async function logout() {
		await fetch('/logout', {
			method: 'POST'
		});
		window.location.href = '/login';
	}
</script>

<svelte:head>
	<title>Account Suspended - Clio</title>
</svelte:head>

<div class="min-h-screen bg-gradient-to-br from-background to-muted">
	<div class="flex min-h-screen items-center justify-center p-4">
		<div class="w-full max-w-md">
			<div class="mb-8 text-center">
				<h1 class="text-3xl font-bold text-foreground">Clio</h1>
				<p class="mt-2 text-muted-foreground">Enterprise Search Platform</p>
			</div>
			
			<Card class="w-full">
				<CardHeader class="text-center">
					<CardTitle class="text-2xl">Account Suspended</CardTitle>
					<CardDescription>Your account has been suspended</CardDescription>
				</CardHeader>
				<CardContent class="text-center space-y-4">
					<div class="rounded-md bg-destructive/10 p-4">
						<div class="text-sm text-destructive">
							Your account has been suspended by an administrator. You cannot access Clio at this time.
						</div>
					</div>
					
					<p class="text-sm text-muted-foreground">
						Please contact your administrator for more information about your account status.
					</p>
					
					<Button on:click={logout} class="w-full">
						Sign out
					</Button>
				</CardContent>
			</Card>
		</div>
	</div>
</div>