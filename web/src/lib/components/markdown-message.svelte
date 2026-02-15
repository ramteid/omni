<script lang="ts">
    import { marked, type Tokens, type RendererObject } from 'marked'
    import { mount } from 'svelte'
    import LinkHoverCard from './reflink-hover-card.svelte'
    import {
        getSourceDisplayName,
        getSourceIconPath,
        getSourceTypeFromDisplayName,
    } from '$lib/utils/icons'
    import { SourceType } from '$lib/types'
    import type { TextCitationParam } from '@anthropic-ai/sdk/resources.js'
    import type { CitationSearchResultLocationParam } from '@anthropic-ai/sdk/resources'

    type Props = {
        content: string
        citations?: TextCitationParam[]
    }

    let { content, citations }: Props = $props()
    let containerRef: HTMLElement | undefined = $state()

    const renderer: RendererObject = {
        link({ href, tokens }: Tokens.Link): string {
            const citation = citations?.find(
                (c) => c.type === 'search_result_location' && c.source === href,
            ) as CitationSearchResultLocationParam | null
            const text = this.parser.parseInline(tokens)
            return `<a href="${href}" class="omni-reflink" title="${citation?.title}" data-snippet="${citation?.cited_text}">${text}</a>`
        },
    }

    marked.use({ renderer })

    $effect(() => {
        if (!containerRef) {
            return
        }

        containerRef.innerHTML = marked.parse(content, { async: false })

        const linkPlaceholders = containerRef.querySelectorAll('.omni-reflink')
        linkPlaceholders.forEach((link) => {
            const href = link.getAttribute('href')
            const title = link.getAttribute('title')
            const text = link.textContent
            const snippet = link.getAttribute('data-snippet')

            mount(LinkHoverCard, {
                target: link.parentNode as Element,
                anchor: link,
                props: {
                    href: href || '#',
                    title: title || '',
                    text,
                    snippet: snippet || undefined,
                },
            })

            link.remove()
        })

        // Inject app icons inline before the first occurrence of each recognized app name
        const appNames = Object.values(SourceType)
            .map((st) => getSourceDisplayName(st))
            .filter((name): name is string => !!name)
            .sort((a, b) => b.length - a.length)
        const injectedApps = new Set<string>()

        const isInsideSkippedTag = (node: Node): boolean => {
            let ancestor = node.parentElement
            while (ancestor && ancestor !== containerRef) {
                const tag = ancestor.tagName.toLowerCase()
                if (tag === 'a' || tag === 'code' || tag === 'pre') return true
                ancestor = ancestor.parentElement
            }
            return false
        }

        const processTextNode = (textNode: Text) => {
            if (isInsideSkippedTag(textNode)) return

            for (const appName of appNames) {
                if (injectedApps.has(appName)) continue

                const textLower = textNode.textContent?.toLowerCase() ?? ''
                const idx = textLower.indexOf(appName.toLowerCase())
                if (idx === -1) continue

                // Split: [before] | [appName + rest]
                const afterNode = textNode.splitText(idx)
                // Split: [appName] | [rest]
                const restNode = afterNode.splitText(appName.length)
                const matchedText = afterNode.textContent ?? appName

                const sourceType = getSourceTypeFromDisplayName(matchedText)
                const iconSrc = sourceType ? getSourceIconPath(sourceType) : null
                if (!iconSrc) continue

                const img = document.createElement('img')
                img.src = iconSrc
                img.alt = ''
                img.style.display = 'inline'
                img.style.height = '1em'
                img.style.width = '1em'
                img.style.verticalAlign = 'sub'
                img.style.marginRight = '0.15em'
                img.style.marginTop = '0em'
                img.style.marginBottom = '0em'

                const bold = document.createElement('strong')
                bold.textContent = appName

                const parent = afterNode.parentNode!
                parent.insertBefore(img, afterNode)
                parent.replaceChild(bold, afterNode)
                injectedApps.add(appName)

                processTextNode(restNode)
            }
        }

        const walker = document.createTreeWalker(containerRef, NodeFilter.SHOW_TEXT)
        const textNodes: Text[] = []
        let node: Text | null
        while ((node = walker.nextNode() as Text | null)) {
            textNodes.push(node)
        }
        for (const textNode of textNodes) {
            processTextNode(textNode)
        }
    })
</script>

<div bind:this={containerRef}></div>
