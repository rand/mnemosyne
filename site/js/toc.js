// Table of Contents generation (optional enhancement)
// Projects can use this to auto-generate TOCs from h2/h3 headings

(function() {
    function generateTOC() {
        const tocContainer = document.querySelector('#toc');
        if (!tocContainer) return;

        const headings = document.querySelectorAll('h2[id], h3[id]');
        if (headings.length === 0) return;

        const tocList = document.createElement('ul');
        tocList.className = 'toc-list';

        let currentLevel = 2;
        let currentList = tocList;
        const listStack = [tocList];

        headings.forEach(heading => {
            const level = parseInt(heading.tagName.substring(1));
            const item = document.createElement('li');
            const link = document.createElement('a');

            link.href = '#' + heading.id;
            link.textContent = heading.textContent;
            link.className = 'toc-link';

            item.appendChild(link);

            // Handle nesting for h3 under h2
            if (level > currentLevel) {
                const nestedList = document.createElement('ul');
                nestedList.className = 'toc-nested';
                currentList.lastElementChild.appendChild(nestedList);
                listStack.push(nestedList);
                currentList = nestedList;
            } else if (level < currentLevel) {
                listStack.pop();
                currentList = listStack[listStack.length - 1];
            }

            currentList.appendChild(item);
            currentLevel = level;
        });

        tocContainer.appendChild(tocList);
    }

    // Run on DOMContentLoaded
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', generateTOC);
    } else {
        generateTOC();
    }
})();
