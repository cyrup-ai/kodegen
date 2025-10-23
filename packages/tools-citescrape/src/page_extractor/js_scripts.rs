//! JavaScript evaluation scripts
//!
//! This module contains the JavaScript code used to extract various types
//! of data from web pages.

/// JavaScript script to extract page metadata
pub const METADATA_SCRIPT: &str = r#"
    (() => {
        const meta = {};
        document.querySelectorAll('meta').forEach(tag => {
            const name = tag.getAttribute('name') || tag.getAttribute('property');
            if (name) {
                meta[name] = tag.getAttribute('content');
            }
        });

        return {
            description: meta['description'] || meta['og:description'] || null,
            keywords: meta['keywords'] ? meta['keywords'].split(',').map(k => k.trim()) : [],
            author: meta['author'] || meta['og:author'] || null,
            published_date: meta['article:published_time'] || meta['publishedDate'] || null,
            modified_date: meta['article:modified_time'] || meta['modifiedDate'] || null,
            language: document.documentElement.lang || null,
            canonical_url: document.querySelector('link[rel="canonical"]')?.href || null,
            robots: meta['robots'] || null,
            viewport: meta['viewport'] || null,
            headers: {}
        };
    })()
"#;

/// JavaScript script to extract page resources
pub const RESOURCES_SCRIPT: &str = r#"
    (() => {
        const scripts = Array.from(document.getElementsByTagName('script')).map(script => ({
            url: script.src || null,
            inline: !script.src,
            async_load: script.async,
            defer: script.defer,
            content_hash: null
        }));

        const stylesheets = Array.from(document.getElementsByTagName('link'))
            .filter(link => link.rel === 'stylesheet')
            .map(style => ({
                url: style.href || null,
                inline: !style.href,
                media: style.media || null,
                content_hash: null
            }));

        const images = Array.from(document.getElementsByTagName('img')).map(img => ({
            url: img.src,
            alt: img.alt || null,
            dimensions: img.width && img.height ? [img.width, img.height] : null,
            size_bytes: null,
            format: img.src.split('.').pop()?.split('?')[0] || null
        }));

        const media = Array.from(document.querySelectorAll('video, audio'))
            .map(media => {
                const source = media.querySelector('source');
                const url = media.src || source?.src;
                return url ? {
                    url: url,
                    media_type: media.tagName.toLowerCase(),
                    format: url.split('.').pop()?.split('?')[0] || null,
                    duration: media.duration || null,
                    size_bytes: null
                } : null;
            })
            .filter(media => media !== null);

        const fonts = Array.from(document.querySelectorAll('link[rel="preload"][as="font"], link[rel="font"]')).map(font => ({
            url: font.href || null,
            format: font.getAttribute('type') || null,
            family: font.getAttribute('font-family') || 'unknown',
            weight: font.getAttribute('font-weight') ? parseInt(font.getAttribute('font-weight')) : null,
            style: font.getAttribute('font-style') || null
        }));

        return {
            scripts,
            stylesheets,
            images,
            media,
            fonts
        };
    })()
"#;

/// JavaScript script to extract timing information
pub const TIMING_SCRIPT: &str = r"
    (() => {
        const timing = performance.timing || {};
        const nav = performance.getEntriesByType('navigation')[0] || {};
        
        return {
            navigation_start: timing.navigationStart || nav.startTime || 0,
            response_end: timing.responseEnd || nav.responseEnd || 0,
            dom_complete: timing.domComplete || nav.domComplete || 0,
            load_complete: timing.loadEventEnd || nav.loadEventEnd || 0,
            total_duration: (timing.loadEventEnd || nav.loadEventEnd || 0) - 
                        (timing.navigationStart || nav.startTime || 0)
        };
    })()
";

/// JavaScript script to extract security information
pub const SECURITY_SCRIPT: &str = r#"
    (() => {
        const url = new URL(window.location.href);
        const cert = window.performance.getEntriesByType('resource')
            .find(entry => entry.name === window.location.href);

        return {
            https: url.protocol === 'https:',
            hsts: document.location.protocol === 'https:' && 
                document.querySelector('meta[http-equiv="Strict-Transport-Security"]') !== null,
            csp: document.querySelector('meta[http-equiv="Content-Security-Policy"]')?.content || '',
            x_frame_options: document.querySelector('meta[http-equiv="X-Frame-Options"]')?.content || '',
            permissions_policy: document.querySelector('meta[http-equiv="Permissions-Policy"]')?.content || ''
        };
    })()
"#;

/// JavaScript script to extract interactive elements with unique data attributes
pub const INTERACTIVE_ELEMENTS_SCRIPT: &str = r#"
    (() => {
        // Comprehensive interactive elements query
        const selector = [
            // Standard form controls
            'button', 'input', 'select', 'textarea',
            
            // Links
            'a[href]',
            
            // Native interactive HTML elements
            'details', 'summary', 'dialog', 'menu',
            
            // Labels (interactive when associated with input)
            'label[for]',
            
            // Event handlers (any element with these is interactive)
            '[onclick]', '[onsubmit]', '[onchange]', '[ondblclick]',
            
            // Interactive ARIA roles
            '[role="button"]',
            '[role="checkbox"]',
            '[role="radio"]',
            '[role="switch"]',
            '[role="tab"]',
            '[role="slider"]',
            '[role="spinbutton"]',
            '[role="menuitem"]',
            '[role="menuitemcheckbox"]',
            '[role="menuitemradio"]',
            '[role="option"]',
            '[role="link"]',
            '[role="searchbox"]',
            '[role="textbox"]',
            '[role="combobox"]',
            '[role="gridcell"]',
            '[role="treeitem"]',
            
            // Other interactive attributes
            '[contenteditable="true"]',
            '[draggable="true"]',
            '[tabindex]'
        ].join(', ');
        
        const elements = document.querySelectorAll(selector);
        
        return Array.from(elements).map((el, index) => {
            // Generate unique data attribute for guaranteed valid selector
            const uniqueId = `interactive-${index}`;
            el.setAttribute('data-citescrape-interactive', uniqueId);
            
            return {
                element_type: el.tagName.toLowerCase(),
                selector: `[data-citescrape-interactive="${uniqueId}"]`,
                text: el.textContent?.trim() || null,
                url: el.href || null,
                attributes: Object.fromEntries(
                    Array.from(el.attributes).map(attr => [attr.name, attr.value])
                )
            };
        });
    })()
"#;

/// JavaScript script to extract links
pub const LINKS_SCRIPT: &str = r"
    (() => {
        const currentUrl = new URL(window.location.href);
        const links = Array.from(document.querySelectorAll('a[href]'))
            .map(link => {
                const href = link.getAttribute('href');
                if (!href) return null;
                
                try {
                    // Handle relative URLs
                    const absoluteUrl = new URL(href, window.location.href);
                    
                    // Only return http/https links
                    if (!['http:', 'https:'].includes(absoluteUrl.protocol)) {
                        return null;
                    }
                    
                    return {
                        url: absoluteUrl.href,
                        text: link.textContent?.trim() || '',
                        title: link.getAttribute('title') || '',
                        rel: link.getAttribute('rel') || '',
                        is_external: absoluteUrl.host !== currentUrl.host,
                        path: absoluteUrl.pathname
                    };
                } catch (e) {
                    return null;
                }
            })
            .filter(link => link !== null);
            
        // Remove duplicates by URL
        const uniqueLinks = [];
        const seenUrls = new Set();
        
        for (const link of links) {
            if (!seenUrls.has(link.url)) {
                seenUrls.add(link.url);
                uniqueLinks.push(link);
            }
        }
        
        return uniqueLinks;
    })()
";
