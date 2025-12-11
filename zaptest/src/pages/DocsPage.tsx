import { useState, useEffect } from 'react';
import DocsLayout from '../components/docs/DocsLayout';
import { docSections } from '../components/docs/DocsSections';

export default function DocsPage() {
  // Get section from URL hash or default to 'introduction'
  const [currentSection, setCurrentSection] = useState(() => {
    const hash = window.location.hash.slice(1);
    return docSections.find(s => s.id === hash)?.id || 'introduction';
  });

  // Update URL hash when section changes
  useEffect(() => {
    window.location.hash = currentSection;
  }, [currentSection]);

  // Listen for hash changes (back/forward navigation)
  useEffect(() => {
    const handleHashChange = () => {
      const hash = window.location.hash.slice(1);
      const section = docSections.find(s => s.id === hash);
      if (section) {
        setCurrentSection(section.id);
      }
    };

    window.addEventListener('hashchange', handleHashChange);
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  // Scroll to top when section changes
  useEffect(() => {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }, [currentSection]);

  return (
    <DocsLayout
      sections={docSections}
      currentSection={currentSection}
      onSectionChange={setCurrentSection}
    />
  );
}
