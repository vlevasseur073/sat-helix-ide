// Custom JavaScript for sat-helix-ide documentation

document.addEventListener('DOMContentLoaded', function() {
    // Initialize mermaid diagrams if the library is loaded
    if (typeof mermaid !== 'undefined') {
        mermaid.initialize({
            startOnLoad: true,
            theme: 'default',
            flowchart: {
                useMaxWidth: true,
                htmlLabels: true,
                curve: 'basis'
            },
            sequence: {
                useMaxWidth: true
            },
            gantt: {
                useMaxWidth: true
            }
        });

        // Re-render mermaid diagrams after page load
        mermaid.init(undefined, document.querySelectorAll('.mermaid'));
    }
});

// Smooth scrolling for anchor links
function smoothScrollToAnchor() {
    const hash = window.location.hash;
    if (hash) {
        const element = document.querySelector(hash);
        if (element) {
            element.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    }
}

// Call smooth scrolling on page load
document.addEventListener('DOMContentLoaded', smoothScrollToAnchor);

// Also handle dynamic content loading (if any)
if (typeof MutationObserver !== 'undefined') {
    const observer = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            if (mutation.addedNodes.length) {
                smoothScrollToAnchor();
            }
        });
    });

    observer.observe(document.body, {
        childList: true,
        subtree: true
    });
}

// Add keyboard shortcuts for navigation
document.addEventListener('keydown', function(event) {
    // Don't interfere with input fields
    if (event.target.tagName === 'INPUT' || event.target.tagName === 'TEXTAREA') {
        return;
    }

    // Next chapter (Right arrow or L)
    if (event.key === 'ArrowRight' || event.key === 'l' || event.key === 'L') {
        const nextLink = document.querySelector('.nav-chapters-next a');
        if (nextLink) {
            nextLink.click();
            event.preventDefault();
        }
    }

    // Previous chapter (Left arrow or H)
    if (event.key === 'ArrowLeft' || event.key === 'h' || event.key === 'H') {
        const prevLink = document.querySelector('.nav-chapters-previous a');
        if (prevLink) {
            prevLink.click();
            event.preventDefault();
        }
    }
});

// Add click handlers for terminal mockups (if they exist)
document.addEventListener('click', function(event) {
    const terminalMockup = event.target.closest('pre.terminal-mockup');
    if (terminalMockup) {
        // Add visual feedback
        terminalMockup.style.transform = 'scale(0.99)';
        setTimeout(() => {
            terminalMockup.style.transform = 'scale(1)';
        }, 150);
    }
});

// Syntax highlighting enhancement for code blocks
function enhanceCodeBlocks() {
    const codeBlocks = document.querySelectorAll('pre code');
    codeBlocks.forEach(function(codeBlock) {
        // Add line numbers if not present
        if (!codeBlock.classList.contains('line-numbers')) {
            const lines = codeBlock.textContent.split('\n');
            if (lines.length > 1) {
                codeBlock.classList.add('line-numbers');
            }
        }

        // Add copy button to code blocks
        const pre = codeBlock.parentElement;
        if (pre.tagName === 'PRE' && !pre.querySelector('.copy-btn')) {
            const copyBtn = document.createElement('button');
            copyBtn.className = 'copy-btn';
            copyBtn.textContent = 'Copy';
            copyBtn.title = 'Copy code to clipboard';

            copyBtn.addEventListener('click', function() {
                const text = codeBlock.textContent;
                navigator.clipboard.writeText(text).then(function() {
                    copyBtn.textContent = 'Copied!';
                    setTimeout(() => {
                        copyBtn.textContent = 'Copy';
                    }, 2000);
                }).catch(function(err) {
                    console.error('Failed to copy: ', err);
                });
            });

            pre.style.position = 'relative';
            copyBtn.style.position = 'absolute';
            copyBtn.style.top = '0.5em';
            copyBtn.style.right = '0.5em';
            copyBtn.style.padding = '0.25em 0.5em';
            copyBtn.style.fontSize = '0.8em';
            copyBtn.style.backgroundColor = '#3498db';
            copyBtn.style.color = 'white';
            copyBtn.style.border = 'none';
            copyBtn.style.borderRadius = '4px';
            copyBtn.style.cursor = 'pointer';
            copyBtn.style.opacity = '0.8';

            copyBtn.addEventListener('mouseenter', function() {
                copyBtn.style.opacity = '1';
            });

            copyBtn.addEventListener('mouseleave', function() {
                copyBtn.style.opacity = '0.8';
            });

            pre.appendChild(copyBtn);
        }
    });
}

// Call code block enhancement after page load
document.addEventListener('DOMContentLoaded', enhanceCodeBlocks);

// Also handle dynamically loaded content
if (typeof MutationObserver !== 'undefined') {
    const contentObserver = new MutationObserver(function(mutations) {
        mutations.forEach(function(mutation) {
            if (mutation.addedNodes.length) {
                enhanceCodeBlocks();
                smoothScrollToAnchor();
            }
        });
    });

    contentObserver.observe(document.querySelector('main') || document.body, {
        childList: true,
        subtree: true
    });
}

// Table of contents sticky navigation
function initStickyTOC() {
    const sidebar = document.getElementById('sidebar');
    const header = document.querySelector('header');

    if (sidebar && header) {
        const observer = new IntersectionObserver(function(entries) {
            entries.forEach(function(entry) {
                if (entry.isIntersecting) {
                    sidebar.classList.remove('sticky');
                } else {
                    sidebar.classList.add('sticky');
                }
            });
        }, { threshold: 1.0 });

        observer.observe(header);
    }
}

// Initialize sticky TOC
document.addEventListener('DOMContentLoaded', initStickyTOC);

// Add loading animation for images
function initImageLoading() {
    const images = document.querySelectorAll('img');
    images.forEach(function(img) {
        if (!img.complete) {
            img.style.opacity = '0.5';
            img.style.transition = 'opacity 0.3s ease';

            img.addEventListener('load', function() {
                this.style.opacity = '1';
            });

            img.addEventListener('error', function() {
                this.style.opacity = '1';
                this.style.border = '1px solid #e74c3c';
            });
        }
    });
}

// Initialize image loading
document.addEventListener('DOMContentLoaded', initImageLoading);

// Auto-hide flash messages
function initFlashMessages() {
    const flashes = document.querySelectorAll('.flash-message');
    flashes.forEach(function(flash) {
        setTimeout(function() {
            flash.style.transition = 'opacity 0.5s ease';
            flash.style.opacity = '0';
            setTimeout(function() {
                flash.remove();
            }, 500);
        }, 5000);
    });
}

// Initialize flash messages
document.addEventListener('DOMContentLoaded', initFlashMessages);

// Improved keyboard navigation for tables
function initTableNavigation() {
    const tables = document.querySelectorAll('table');
    tables.forEach(function(table) {
        table.addEventListener('keydown', function(event) {
            const cells = table.querySelectorAll('td, th');
            const currentIndex = Array.from(cells).indexOf(document.activeElement);

            if (currentIndex === -1) return;

            switch (event.key) {
                case 'ArrowRight':
                    if (currentIndex + 1 < cells.length) {
                        cells[currentIndex + 1].focus();
                        event.preventDefault();
                    }
                    break;
                case 'ArrowLeft':
                    if (currentIndex - 1 >= 0) {
                        cells[currentIndex - 1].focus();
                        event.preventDefault();
                    }
                    break;
                case 'ArrowDown':
                    const currentRow = Array.from(table.rows).findIndex(row =>
                        Array.from(row.cells).includes(document.activeElement)
                    );
                    if (currentRow + 1 < table.rows.length) {
                        const nextRow = table.rows[currentRow + 1];
                        const currentCellIndex = Array.from(table.rows[currentRow].cells).indexOf(document.activeElement);
                        if (currentCellIndex < nextRow.cells.length) {
                            nextRow.cells[currentCellIndex].focus();
                            event.preventDefault();
                        }
                    }
                    break;
                case 'ArrowUp':
                    const currentRowUp = Array.from(table.rows).findIndex(row =>
                        Array.from(row.cells).includes(document.activeElement)
                    );
                    if (currentRowUp - 1 >= 0) {
                        const prevRow = table.rows[currentRowUp - 1];
                        const currentCellIndex = Array.from(table.rows[currentRowUp].cells).indexOf(document.activeElement);
                        if (currentCellIndex < prevRow.cells.length) {
                            prevRow.cells[currentCellIndex].focus();
                            event.preventDefault();
                        }
                    }
                    break;
            }
        });

        // Make table cells focusable
        const cells = table.querySelectorAll('td, th');
        cells.forEach(function(cell) {
            cell.setAttribute('tabindex', '0');
        });
    });
}

// Initialize table navigation
document.addEventListener('DOMContentLoaded', initTableNavigation);

// Add smooth scrolling for sidebar navigation
function initSidebarNavigation() {
    const sidebarLinks = document.querySelectorAll('#sidebar a[href^="#"]');
    sidebarLinks.forEach(function(link) {
        link.addEventListener('click', function(event) {
            const hash = this.getAttribute('href');
            if (hash) {
                const target = document.querySelector(hash);
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                    // Update URL without jumping
                    history.pushState(null, null, hash);
                    event.preventDefault();
                }
            }
        });
    });
}

// Initialize sidebar navigation
document.addEventListener('DOMContentLoaded', initSidebarNavigation);

// Highlight current section in sidebar
function initCurrentSectionHighlight() {
    const sections = document.querySelectorAll('h2, h3, h4');
    const sidebarLinks = document.querySelectorAll('#sidebar a[href^="#"]');

    function updateActiveLink() {
        let current = '';

        sections.forEach(function(section) {
            const sectionTop = section.getBoundingClientRect().top;
            const sectionId = section.getAttribute('id');

            if (sectionTop < 100) {
                current = '#' + sectionId;
            }
        });

        sidebarLinks.forEach(function(link) {
            link.classList.remove('active');
            if (link.getAttribute('href') === current) {
                link.classList.add('active');
            }
        });
    }

    // Update on scroll
    window.addEventListener('scroll', updateActiveLink);

    // Update on load
    updateActiveLink();
}

// Initialize current section highlighting
document.addEventListener('DOMContentLoaded', initCurrentSectionHighlight);

// Add touch support for mobile devices
function initTouchSupport() {
    let touchStartX = 0;
    let touchStartY = 0;

    document.addEventListener('touchstart', function(event) {
        touchStartX = event.touches[0].clientX;
        touchStartY = event.touches[0].clientY;
    }, { passive: true });

    document.addEventListener('touchmove', function(event) {
        const touchEndX = event.touches[0].clientX;
        const touchEndY = event.touches[0].clientY;
        const diffX = touchStartX - touchEndX;
        const diffY = touchStartY - touchEndY;

        // Horizontal swipe for navigation
        if (Math.abs(diffX) > Math.abs(diffY) && Math.abs(diffX) > 50) {
            if (diffX > 0) {
                // Left swipe - next
                const nextLink = document.querySelector('.nav-chapters-next a');
                if (nextLink) {
                    nextLink.click();
                    event.preventDefault();
                }
            } else {
                // Right swipe - previous
                const prevLink = document.querySelector('.nav-chapters-previous a');
                if (prevLink) {
                    prevLink.click();
                    event.preventDefault();
                }
            }
        }

        touchStartX = 0;
        touchStartY = 0;
    }, { passive: true });
}

// Initialize touch support
document.addEventListener('DOMContentLoaded', initTouchSupport);

// Add loading indicator for slow operations
function initLoadingIndicators() {
    // This can be extended if needed for dynamic content loading
    const loadingElements = document.querySelectorAll('.loading');
    loadingElements.forEach(function(element) {
        element.style.display = 'none';
    });
}

// Initialize loading indicators
document.addEventListener('DOMContentLoaded', initLoadingIndicators);

// Console welcome message
console.log('%c sat-helix-ide Documentation ', 'background: #2c3e50; color: white; font-size: 16px; padding: 5px;');
console.log('%c Welcome to the sat-helix-ide User Manual & Design Document! ', 'color: #3498db; font-size: 14px;');
console.log('%c For keyboard shortcuts, see the User Manual. ', 'color: #666; font-size: 12px;');
