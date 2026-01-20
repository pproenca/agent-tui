import React, { useState } from 'react';
import { render, Box, Text, useInput, useApp } from 'ink';
import TextInput from 'ink-text-input';
import SelectInput from 'ink-select-input';

interface FormData {
  name: string;
  language: string;
  useTypeScript: boolean;
}

const App: React.FC = () => {
  const { exit } = useApp();
  const [step, setStep] = useState<'name' | 'language' | 'typescript' | 'confirm' | 'done'>('name');
  const [formData, setFormData] = useState<FormData>({
    name: '',
    language: '',
    useTypeScript: false,
  });

  const languages = [
    { label: 'JavaScript', value: 'javascript' },
    { label: 'Python', value: 'python' },
    { label: 'Rust', value: 'rust' },
    { label: 'Go', value: 'go' },
  ];

  const boolOptions = [
    { label: 'Yes', value: 'yes' },
    { label: 'No', value: 'no' },
  ];

  const handleNameSubmit = (value: string) => {
    setFormData({ ...formData, name: value });
    setStep('language');
  };

  const handleLanguageSelect = (item: { value: string }) => {
    setFormData({ ...formData, language: item.value });
    setStep('typescript');
  };

  const handleTypeScriptSelect = (item: { value: string }) => {
    setFormData({ ...formData, useTypeScript: item.value === 'yes' });
    setStep('confirm');
  };

  useInput((input, key) => {
    if (step === 'confirm') {
      if (input === 'y' || key.return) {
        setStep('done');
        setTimeout(() => exit(), 1000);
      } else if (input === 'n') {
        setStep('name');
        setFormData({ name: '', language: '', useTypeScript: false });
      }
    }
  });

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text bold>Simple Form Test</Text>
      </Box>

      {step === 'name' && (
        <Box>
          <Text>What is your project name? </Text>
          <TextInput
            value={formData.name}
            onChange={(value) => setFormData({ ...formData, name: value })}
            onSubmit={handleNameSubmit}
          />
        </Box>
      )}

      {step === 'language' && (
        <Box flexDirection="column">
          <Text>Select a language:</Text>
          <SelectInput items={languages} onSelect={handleLanguageSelect} />
        </Box>
      )}

      {step === 'typescript' && (
        <Box flexDirection="column">
          <Text>Use TypeScript?</Text>
          <SelectInput items={boolOptions} onSelect={handleTypeScriptSelect} />
        </Box>
      )}

      {step === 'confirm' && (
        <Box flexDirection="column">
          <Text>Review your choices:</Text>
          <Box marginLeft={2} flexDirection="column">
            <Text>Project: {formData.name}</Text>
            <Text>Language: {formData.language}</Text>
            <Text>TypeScript: {formData.useTypeScript ? 'Yes' : 'No'}</Text>
          </Box>
          <Box marginTop={1}>
            <Text>Confirm? (y/n)</Text>
          </Box>
        </Box>
      )}

      {step === 'done' && (
        <Box>
          <Text color="green">✓ Form submitted successfully!</Text>
        </Box>
      )}
    </Box>
  );
};

render(<App />);
