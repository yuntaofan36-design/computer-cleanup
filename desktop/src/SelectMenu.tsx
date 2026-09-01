import {
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { CSSProperties, KeyboardEvent, ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown } from 'lucide-react';

export interface SelectMenuOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface SelectMenuProps {
  value: string;
  options: readonly SelectMenuOption[];
  onChange: (value: string) => void;
  ariaLabel: string;
  label?: string;
  leading?: ReactNode;
  disabled?: boolean;
  placeholder?: string;
  className?: string;
}

interface MenuPosition {
  top: number;
  left: number;
  width: number;
  placement: 'top' | 'bottom';
}

function enabledIndex(
  options: readonly SelectMenuOption[],
  start: number,
  direction: 1 | -1,
): number {
  if (!options.length) return -1;
  for (let offset = 1; offset <= options.length; offset += 1) {
    const index = (start + direction * offset + options.length) % options.length;
    if (!options[index]?.disabled) return index;
  }
  return -1;
}

function edgeIndex(options: readonly SelectMenuOption[], fromEnd = false): number {
  const start = fromEnd ? options.length - 1 : 0;
  const end = fromEnd ? -1 : options.length;
  const step = fromEnd ? -1 : 1;
  for (let index = start; index !== end; index += step) {
    if (!options[index]?.disabled) return index;
  }
  return -1;
}

export default function SelectMenu({
  value,
  options,
  onChange,
  ariaLabel,
  label,
  leading,
  disabled = false,
  placeholder = '请选择',
  className = '',
}: SelectMenuProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const listboxId = useId();
  const selectedIndex = useMemo(
    () => options.findIndex((option) => option.value === value),
    [options, value],
  );
  const selected = selectedIndex >= 0 ? options[selectedIndex] : undefined;
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(selectedIndex);
  const [position, setPosition] = useState<MenuPosition>({
    top: 0,
    left: 0,
    width: 168,
    placement: 'bottom',
  });

  const updatePosition = () => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const width = Math.max(168, rect.width);
    const estimatedHeight = Math.min(280, Math.max(48, options.length * 38 + 12));
    const measuredHeight = menuRef.current?.offsetHeight || estimatedHeight;
    const roomBelow = window.innerHeight - rect.bottom;
    const placement = roomBelow < measuredHeight + 12 && rect.top > roomBelow ? 'top' : 'bottom';
    const top = placement === 'top'
      ? Math.max(8, rect.top - measuredHeight - 6)
      : Math.min(window.innerHeight - measuredHeight - 8, rect.bottom + 6);
    const left = Math.min(
      Math.max(8, rect.left),
      Math.max(8, window.innerWidth - width - 8),
    );
    setPosition({ top, left, width, placement });
  };

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
  }, [open, options.length]);

  useEffect(() => {
    if (!open) return undefined;
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!triggerRef.current?.contains(target) && !menuRef.current?.contains(target)) {
        setOpen(false);
      }
    };
    const reposition = () => updatePosition();
    document.addEventListener('pointerdown', closeOutside);
    window.addEventListener('resize', reposition);
    window.addEventListener('scroll', reposition, true);
    return () => {
      document.removeEventListener('pointerdown', closeOutside);
      window.removeEventListener('resize', reposition);
      window.removeEventListener('scroll', reposition, true);
    };
  }, [open, options.length]);

  useEffect(() => {
    if (disabled && open) setOpen(false);
  }, [disabled, open]);

  useEffect(() => {
    if (!open || activeIndex < 0) return;
    const activeOption = document.getElementById(`${listboxId}-option-${activeIndex}`);
    if (activeOption && menuRef.current?.contains(activeOption) && typeof activeOption.scrollIntoView === 'function') {
      activeOption.scrollIntoView({ block: 'nearest' });
    }
  }, [activeIndex, listboxId, open]);

  const openMenu = () => {
    if (disabled || !options.length) return;
    setActiveIndex(selectedIndex >= 0 ? selectedIndex : edgeIndex(options));
    setOpen(true);
  };

  const choose = (index: number) => {
    const option = options[index];
    if (!option || option.disabled) return;
    if (option.value !== value) onChange(option.value);
    setOpen(false);
    triggerRef.current?.focus();
  };

  const move = (direction: 1 | -1) => {
    const start = activeIndex >= 0 ? activeIndex : selectedIndex;
    setActiveIndex(enabledIndex(options, start, direction));
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      if (!open) {
        openMenu();
      } else {
        move(event.key === 'ArrowDown' ? 1 : -1);
      }
      return;
    }
    if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      if (!open) openMenu();
      setActiveIndex(edgeIndex(options, event.key === 'End'));
      return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (open) choose(activeIndex);
      else openMenu();
      return;
    }
    if (event.key === 'Escape' && open) {
      event.preventDefault();
      setOpen(false);
      return;
    }
    if (event.key === 'Tab') {
      setOpen(false);
      return;
    }
    if (event.key.length === 1 && /\S/.test(event.key)) {
      const query = event.key.toLocaleLowerCase();
      const start = activeIndex >= 0 ? activeIndex : selectedIndex;
      for (let offset = 1; offset <= options.length; offset += 1) {
        const index = (start + offset + options.length) % options.length;
        const option = options[index];
        if (!option?.disabled && option.label.toLocaleLowerCase().startsWith(query)) {
          event.preventDefault();
          if (!open) openMenu();
          setActiveIndex(index);
          break;
        }
      }
    }
  };

  const menuStyle: CSSProperties = {
    top: position.top,
    left: position.left,
    width: position.width,
  };

  return <div className={`lumina-select ${className} ${open ? 'open' : ''} ${disabled ? 'disabled' : ''}`.trim()}>
    <button
      ref={triggerRef}
      type="button"
      role="combobox"
      className="lumina-select-trigger"
      aria-label={ariaLabel}
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-controls={listboxId}
      aria-activedescendant={open && activeIndex >= 0 ? `${listboxId}-option-${activeIndex}` : undefined}
      disabled={disabled}
      onClick={() => open ? setOpen(false) : openMenu()}
      onKeyDown={handleKeyDown}
    >
      {leading && <span className="lumina-select-leading">{leading}</span>}
      <span className="lumina-select-copy">
        {label && <small>{label}</small>}
        <strong>{selected?.label || placeholder}</strong>
      </span>
      <ChevronDown className="lumina-select-chevron" aria-hidden="true" />
    </button>
    {open && createPortal(
      <div
        ref={menuRef}
        id={listboxId}
        role="listbox"
        className="lumina-select-menu"
        data-placement={position.placement}
        aria-label={ariaLabel}
        style={menuStyle}
      >
        {options.map((option, index) => {
          const optionSelected = index === selectedIndex;
          const active = index === activeIndex;
          return <button
            type="button"
            role="option"
            id={`${listboxId}-option-${index}`}
            className={`lumina-select-option ${optionSelected ? 'selected' : ''} ${active ? 'active' : ''}`.trim()}
            aria-selected={optionSelected}
            disabled={option.disabled}
            tabIndex={-1}
            key={option.value}
            onMouseDown={(event) => event.preventDefault()}
            onMouseEnter={() => !option.disabled && setActiveIndex(index)}
            onClick={() => choose(index)}
          >
            <span>{option.label}</span>
            <Check aria-hidden="true" />
          </button>;
        })}
      </div>,
      document.body,
    )}
  </div>;
}
