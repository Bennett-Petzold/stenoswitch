Split stenography keyboard supporting USB, Bluetooth, battery, and embedded Plover translation.

# Warning!!!
There is no warranty, implied or otherwise, on this repository!
This is a hobby project by someone who has no formal education or training in PCB design. 
I _CANNOT_ guarantee that the battery circuit is safe.
I _CANNOT_ guarantee bluetooth communications comply with safety regulations.
I _AM NOT_ liable if you make this project and it is dysfunctional.
Understand that there is a risk of electrical fire due to the lithium ion battery.
Understand that wireless communications are untested and may interfere with pacemakers etc.

# Capabilities
TODO.

# Licenses
All code is licensed under the Mozilla Public License 2.0.
All non-code is licensed under CC BY-SA 4.0.
Both licenses allow for commercial use, but any commercial modification must be licensed under the same terms.
They will not "infect" other proprietary works.

# Subfolders
Each subfolder 

## Assembling the Project
See [assembly/README.md](assembly/README.md).

## PCB
KiCAD designs for printed circuit board.
See [pcb/README.md](pcb/README.md) for more.

## Software
Hardware-independent software built into the machine image.
Specialization to a specific hardware is handled in yocto.
See [software/README.md](software/README.md).

## Yocto Image
This defines the machine-specific image.
See [yocto/README.md](yocto/README.md).

# Releases
Some release resources are manually uploaded.
The initial install is done with the full disk image.
Subsequent updates are done by flashing ONLY the length of the full disk image.

# Appendix

### Steno Resources
* https://www.openstenoproject.org/plover/
* https://plover.wiki/index.php/Plover_Wiki
* https://lapwing.aerick.ca/
* https://www.artofchording.com/
* https://www.openstenoproject.org/learn-plover/home.html

### Approximate Cost
These are based on my costs (including shipping and taxes) September 2025, US address.
* $35 for keyswitches and lube
* $18 for springs
* $12 for battery
* $17 for Raspberry Pi
* $60 for components from DigiKey
* $50 for 5 PCBs both sides, 1 stencil from JLCPCB
Expect ~ $200 for a full single order.
There are electronics and buying in bulk reduces per-board prices.
Shipping is also a significant part of the price in these low quantities.
