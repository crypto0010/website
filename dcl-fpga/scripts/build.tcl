# Vivado non-project batch build for DCL-FPGA
# Usage: vivado -mode batch -source scripts/build.tcl
# Target: Nexys 4 DDR — XC7A100T-1CSG324C

set project_name "dcl_fpga"
set part "xc7a100tcsg324-1"
set top "dcl_top"

# Source files
set rtl_files [glob rtl/*.v]
set xdc_file "constraints/nexys4ddr.xdc"

# Create output directories
file mkdir reports
file mkdir output

# Create in-memory project
create_project -in_memory -part $part

# Add sources
foreach f $rtl_files {
    read_verilog $f
}
read_xdc $xdc_file

# Synthesis (area-optimized)
synth_design -top $top -part $part -directive AreaOptimized_high
report_utilization -file reports/post_synth_util.rpt -hierarchical
report_timing_summary -file reports/post_synth_timing.rpt

# Implementation
opt_design -directive ExploreArea
place_design
route_design
report_utilization -file reports/post_impl_util.rpt
report_timing_summary -file reports/post_impl_timing.rpt

# Generate bitstream
write_bitstream -force output/${project_name}.bit

puts "=== Build complete: output/${project_name}.bit ==="
